//! Numbers: float arithmetic, int `**`, and conversions between ints,
//! floats and strings. What is not a single instruction is a runtime
//! call (`pythrust-rt/src/num.rs`); one that can fail reports a status
//! byte through a stack slot, which picks the runtime error.

use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{types, InstBuilder, StackSlotData, StackSlotKind, TrapCode, Value};

use super::{backend, runtime, FnGen, Rt, OVERFLOW};
use crate::diag::Result;
use crate::ir::{CmpOp, FloatOp, Operand, Ty};
use crate::span::Span;

/// Marks where a message's run-time part goes.
const MID: char = '\0';
/// `2**63`, the first float above the `i64` range.
const I64_END: f64 = 9_223_372_036_854_775_808.0;

impl FnGen<'_, '_, '_> {
    pub(super) fn float_op(
        &mut self,
        op: FloatOp,
        a: Operand,
        b: Operand,
        span: Span,
    ) -> Result<Value> {
        let (x, y) = (self.operand(a), self.operand(b));
        let by_zero = match op {
            FloatOp::Add => return Ok(self.b.ins().fadd(x, y)),
            FloatOp::Sub => return Ok(self.b.ins().fsub(x, y)),
            FloatOp::Mul => return Ok(self.b.ins().fmul(x, y)),
            FloatOp::Pow => return self.float_pow(x, y, span),
            FloatOp::Div => "ZeroDivisionError: division by zero",
            FloatOp::FloorDiv => "ZeroDivisionError: float floor division by zero",
            FloatOp::Mod => "ZeroDivisionError: float modulo",
        };
        let zero = self.b.ins().f64const(0.0);
        let is_zero = self.b.ins().fcmp(FloatCC::Equal, y, zero);
        self.fail_if(is_zero, span, by_zero)?;
        match op {
            FloatOp::Div => Ok(self.b.ins().fdiv(x, y)),
            FloatOp::FloorDiv => self.call_rt_value(Rt::FloatFloorDiv, &[x, y]),
            _ => self.call_rt_value(Rt::FloatMod, &[x, y]),
        }
    }

    fn float_pow(&mut self, x: Value, y: Value, span: Span) -> Result<Value> {
        let (r, status) = self.with_status(Rt::FloatPow, &[x, y])?;
        let failures = [
            "ZeroDivisionError: 0.0 cannot be raised to a negative power",
            "ValueError: negative number cannot be raised to a fractional power",
            "OverflowError: numerical result out of range",
        ];
        for (code, message) in (1..).zip(failures) {
            self.fail_on(status, code, span, message)?;
        }
        Ok(r)
    }

    pub(super) fn int_pow(&mut self, a: Operand, b: Operand, span: Span) -> Result<Value> {
        let (x, y) = (self.operand(a), self.operand(b));
        let (r, status) = self.with_status(Rt::IntPow, &[x, y])?;
        self.fail_on(status, 1, span, OVERFLOW)?;
        let negative = self.b.ins().icmp_imm_s(IntCC::Equal, status, 2);
        let message = "ValueError: negative exponent in int ** int; use a float base, e.g. \0";
        self.fail_mid_if(negative, span, message, |g| {
            g.call_rt_value(Rt::PowHint, &[x, y])
        })?;
        Ok(r)
    }

    /// An `Int` or `Bool` as a float.
    pub(super) fn int_as_float(&mut self, a: Operand) -> Value {
        let v = self.operand(a);
        let v = match self.func.ty(a) {
            Ty::Bool => self.b.ins().uextend(types::I64, v),
            _ => v,
        };
        self.b.ins().fcvt_from_sint(types::F64, v)
    }

    pub(super) fn float_to_int(&mut self, a: Operand, span: Span) -> Result<Value> {
        let f = self.operand(a);
        let nan = self.b.ins().fcmp(FloatCC::Unordered, f, f);
        self.fail_if(nan, span, "ValueError: cannot convert float NaN to integer")?;
        let (lo, hi) = (
            self.b.ins().f64const(-I64_END),
            self.b.ins().f64const(I64_END),
        );
        let below = self.b.ins().fcmp(FloatCC::LessThan, f, lo);
        let above = self.b.ins().fcmp(FloatCC::GreaterThanOrEqual, f, hi);
        let out = self.b.ins().bor(below, above);
        self.fail_if(
            out,
            span,
            "OverflowError: float too large to convert to int",
        )?;
        Ok(self.b.ins().fcvt_to_sint(types::I64, f))
    }

    pub(super) fn parse_float(&mut self, s: Operand, span: Span) -> Result<Value> {
        let text = self.operand(s);
        let (r, status) = self.with_status(Rt::FloatParse, &[text])?;
        let bad = self.b.ins().icmp_imm_s(IntCC::Equal, status, 1);
        let message = "ValueError: could not convert string to float: \0";
        self.fail_mid_if(bad, span, message, |g| {
            g.call_rt_value(Rt::StrRepr, &[text])
        })?;
        Ok(r)
    }

    pub(super) fn parse_int(&mut self, s: Operand, span: Span) -> Result<Value> {
        let text = self.operand(s);
        let (r, status) = self.with_status(Rt::IntParse, &[text])?;
        let bad = self.b.ins().icmp_imm_s(IntCC::Equal, status, 1);
        let message = "ValueError: invalid literal for int() with base 10: \0";
        self.fail_mid_if(bad, span, message, |g| {
            g.call_rt_value(Rt::StrRepr, &[text])
        })?;
        self.fail_on(status, 2, span, "OverflowError: int too large for i64")?;
        Ok(r)
    }

    /// Calls `rt` with `args` and a pointer to a fresh status byte, giving
    /// the result and the status.
    fn with_status(&mut self, rt: Rt, args: &[Value]) -> Result<(Value, Value)> {
        let data = StackSlotData::new(StackSlotKind::ExplicitSlot, 1, 0);
        let slot = self.b.create_sized_stack_slot(data);
        let addr = self.b.ins().stack_addr(self.cg.ptr, slot, 0);
        let args: Vec<Value> = args.iter().copied().chain([addr]).collect();
        let r = self.call_rt_value(rt, &args)?;
        let status = self.b.ins().stack_load(self.cg.ptr, types::I8, slot, 0);
        Ok((r, status))
    }

    fn fail_on(&mut self, status: Value, code: i64, span: Span, message: &str) -> Result<()> {
        let hit = self.b.ins().icmp_imm_s(IntCC::Equal, status, code);
        self.fail_if(hit, span, message)
    }

    /// Like `fail_if`, for a `message` with a [`MID`] that `mid` fills at
    /// run time with a new string. The rest is rendered now.
    fn fail_mid_if(
        &mut self,
        cond: Value,
        span: Span,
        message: &str,
        mid: impl FnOnce(&mut Self) -> Result<Value>,
    ) -> Result<()> {
        let text = (self.cg.render)(&runtime(span, message));
        let Some((head, tail)) = text.split_once(MID) else {
            return Err(backend(format!("no run-time part in `{message}`")));
        };
        let (head, tail) = (head.to_string(), tail.to_string());
        let (fail, ok) = (self.b.create_block(), self.b.create_block());
        self.b.set_cold_block(fail);
        self.b.ins().brif(cond, fail, &[], ok, &[]);
        self.b.switch_to_block(fail);
        let mid = mid(self)?;
        let (head, head_len) = self.data(head)?;
        let (tail, tail_len) = self.data(tail)?;
        self.call_rt(Rt::FailMid, &[head, head_len, mid, tail, tail_len]);
        self.b.ins().trap(TrapCode::unwrap_user(1));
        self.b.switch_to_block(ok);
        Ok(())
    }
}

/// Python's float comparisons: false with a NaN, but for `!=`.
pub(super) fn float_cc(op: CmpOp) -> FloatCC {
    match op {
        CmpOp::Eq => FloatCC::Equal,
        CmpOp::Ne => FloatCC::NotEqual,
        CmpOp::Lt => FloatCC::LessThan,
        CmpOp::Le => FloatCC::LessThanOrEqual,
        CmpOp::Gt => FloatCC::GreaterThan,
        CmpOp::Ge => FloatCC::GreaterThanOrEqual,
    }
}
