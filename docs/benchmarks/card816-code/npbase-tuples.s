000000000000bbd0 <pythrust_main>:
push   %rbp
mov    %rsp,%rbp
sub    $0x60,%rsp
mov    %rbx,0x30(%rsp)
mov    %r12,0x38(%rsp)
mov    %r13,0x40(%rsp)
mov    %r14,0x48(%rsp)
mov    %r15,0x50(%rsp)
xor    %rbx,%rbx
lea    (%rsp),%r12
lea    0x8(%rsp),%r13
mov    %rbx,0x10(%rsp)
mov    %rbx,0x18(%rsp)
cmp    $0xf4240,%rbx
jl     bc1e <pythrust_main+0x4e>
mov    0x10(%rsp),%rdi
jmp    bd01 <pythrust_main+0x131>
mov    %rdi,%r14
mov    0x8(%r14),%r15
cmp    $0x3e8,%r15
jge    bdc2 <pythrust_main+0x1f2>
lea    0x1(%r15),%rax
mov    %rax,0x8(%r14)
mov    %rbx,%rsi
mov    %r14,%rdi
call   bdd9 <py1_digits>
mov    %rax,0x28(%rsp)
mov    %r15,0x8(%r14)
movzbq (%r14),%rcx
test   %cl,%cl
jne    bd5e <pythrust_main+0x18e>
xor    %rsi,%rsi
lea    0x37bc(%rip),%rcx        # f420 <pt_seq_get>
mov    %r12,%rdx
mov    0x28(%rsp),%rdi
call   *%rcx
mov    %rax,0x20(%rsp)
mov    $0x1,%esi
lea    0x37a1(%rip),%rax        # f420 <pt_seq_get>
mov    %r13,%rdx
mov    0x28(%rsp),%rdi
call   *%rax
mov    %rax,%r15
lea    0x36ad(%rip),%rax        # f340 <pt_seq_dec>
mov    0x28(%rsp),%rdi
call   *%rax
mov    0x20(%rsp),%rax
mov    0x10(%rsp),%rdi
add    %rax,%rdi
seto   %cl
test   %cl,%cl
jne    bd97 <pythrust_main+0x1c7>
mov    %r15,%rax
mov    0x18(%rsp),%rcx
add    %rax,%rcx
mov    %rcx,0x18(%rsp)
seto   %sil
test   %sil,%sil
jne    bd80 <pythrust_main+0x1b0>
mov    %rbx,%rsi
add    $0x1,%rsi
seto   %sil
test   %sil,%sil
jne    bd01 <pythrust_main+0x131>
add    $0x1,%rbx
seto   %r9b
test   %r9b,%r9b
jne    bd68 <pythrust_main+0x198>
mov    %rdi,0x10(%rsp)
mov    %r14,%rdi
jmp    bc07 <pythrust_main+0x37>
lea    0x5c08(%rip),%rax        # 11910 <pt_write_int>
call   *%rax
mov    $0x20,%eax
movzbq %al,%rdi
lea    0x5bc6(%rip),%rax        # 118e0 <pt_write_byte>
call   *%rax
lea    0x5bed(%rip),%rax        # 11910 <pt_write_int>
mov    0x18(%rsp),%rdi
call   *%rax
mov    $0xa,%eax
movzbq %al,%rdi
lea    0x5ba6(%rip),%rax        # 118e0 <pt_write_byte>
call   *%rax
mov    0x30(%rsp),%rbx
mov    0x38(%rsp),%r12
mov    0x40(%rsp),%r13
mov    0x48(%rsp),%r14
mov    0x50(%rsp),%r15
add    $0x60,%rsp
mov    %rbp,%rsp
pop    %rbp
ret
mov    0x28(%rsp),%rdi
jmp    bdae <pythrust_main+0x1de>
lea    0x4032b(%rip),%rdi        # 4c09a <.Ldata1>
mov    $0x90,%esi
lea    0x5ae5(%rip),%r11        # 11860 <pt_fail>
call   *%r11
ud2
lea    0x40427(%rip),%rdi        # 4c1ae <.Ldata3>
mov    $0x84,%esi
lea    0x5acd(%rip),%rax        # 11860 <pt_fail>
call   *%rax
ud2
lea    0x4038c(%rip),%rdi        # 4c12a <.Ldata2>
mov    $0x84,%esi
lea    0x5ab6(%rip),%rax        # 11860 <pt_fail>
call   *%rax
ud2
lea    0x358b(%rip),%rax        # f340 <pt_seq_dec>
call   *%rax
lea    0x6352(%rip),%rax        # 12110 <pt_uncaught>
call   *%rax
ud2
lea    0x40237(%rip),%rdi        # 4c000 <.Ldata0>
mov    $0x9a,%esi
lea    0x5a8b(%rip),%rax        # 11860 <pt_fail>
call   *%rax
ud2
000000000000bdd9 <py1_digits>:
push   %rbp
mov    %rsp,%rbp
sub    $0x60,%rsp
mov    %rbx,0x30(%rsp)
mov    %r12,0x38(%rsp)
mov    %r13,0x40(%rsp)
mov    %r14,0x48(%rsp)
mov    %r15,0x50(%rsp)
xor    %rax,%rax
mov    %rax,%r15
lea    (%rsp),%rbx
lea    0x8(%rsp),%r12
mov    %r15,%r13
mov    %r15,0x10(%rsp)
test   %rsi,%rsi
jg     be5f <py1_digits+0x86>
lea    0x404bd(%rip),%rdi        # 4c2de <.Ldata5>
mov    $0x4,%esi
mov    $0x2,%edx
lea    0x368e(%rip),%rax        # f4c0 <pt_seq_new>
call   *%rax
mov    %rax,%rbx
lea    0x3752(%rip),%rax        # f590 <pt_seq_push>
mov    %r15,%rsi
mov    %rbx,%rdi
call   *%rax
lea    0x3743(%rip),%rax        # f590 <pt_seq_push>
mov    0x10(%rsp),%rsi
mov    %rbx,%rdi
call   *%rax
mov    %r13,%rdi
jmp    bf39 <py1_digits+0x160>
mov    %rdi,%r13
mov    0x8(%r13),%r14
cmp    $0x3e8,%r14
jge    bf97 <py1_digits+0x1be>
lea    0x1(%r14),%rax
mov    %rax,0x8(%r13)
mov    $0xa,%edx
mov    %r13,%rdi
call   bfb0 <py2_div_mod>
mov    %rax,0x20(%rsp)
mov    %r14,0x8(%r13)
movzbq 0x0(%r13),%rcx
test   %cl,%cl
jne    bf28 <py1_digits+0x14f>
xor    %rsi,%rsi
lea    0x3578(%rip),%rcx        # f420 <pt_seq_get>
mov    %rbx,%rdx
mov    0x20(%rsp),%rdi
call   *%rcx
mov    %rax,%r14
mov    $0x1,%esi
lea    0x355f(%rip),%rcx        # f420 <pt_seq_get>
mov    %r12,%rdx
mov    0x20(%rsp),%rdi
call   *%rcx
mov    %rax,0x18(%rsp)
lea    0x3469(%rip),%rcx        # f340 <pt_seq_dec>
mov    0x20(%rsp),%rdi
call   *%rcx
mov    %r15,%rsi
add    $0x1,%rsi
seto   %dil
test   %dil,%dil
jne    bf7f <py1_digits+0x1a6>
mov    0x10(%rsp),%rax
mov    0x18(%rsp),%rcx
add    %rcx,%rax
seto   %dil
test   %dil,%dil
jne    bf67 <py1_digits+0x18e>
xor    %rdi,%rdi
mov    %rax,0x10(%rsp)
mov    %rsi,%r15
mov    %rdi,%rcx
mov    %r13,%rdi
mov    %rcx,%r13
mov    %r14,%rsi
jmp    be11 <py1_digits+0x38>
mov    0x20(%rsp),%rdi
mov    %rax,%rbx
mov    %rax,%r8
xor    %r8,%rax
mov    %rax,%rbx
lea    0x3400(%rip),%rax        # f340 <pt_seq_dec>
call   *%rax
mov    %rbx,%rax
mov    0x30(%rsp),%rbx
mov    0x38(%rsp),%r12
mov    0x40(%rsp),%r13
mov    0x48(%rsp),%r14
mov    0x50(%rsp),%r15
add    $0x60,%rsp
mov    %rbp,%rsp
pop    %rbp
ret
lea    0x403f8(%rip),%rdi        # 4c366 <.Ldata7>
mov    $0x84,%esi
lea    0x58e6(%rip),%r8        # 11860 <pt_fail>
call   *%r8
ud2
lea    0x4035c(%rip),%rdi        # 4c2e2 <.Ldata6>
mov    $0x84,%esi
lea    0x58ce(%rip),%r9        # 11860 <pt_fail>
call   *%r9
ud2
lea    0x40294(%rip),%rdi        # 4c232 <.Ldata4>
mov    $0xac,%esi
lea    0x58b6(%rip),%rax        # 11860 <pt_fail>
call   *%rax
ud2
	...
000000000000bfb0 <py2_div_mod>:
push   %rbp
mov    %rsp,%rbp
sub    $0x50,%rsp
mov    %rbx,0x20(%rsp)
mov    %r12,0x28(%rsp)
mov    %r13,0x30(%rsp)
mov    %r14,0x38(%rsp)
mov    %r15,0x40(%rsp)
test   %rdx,%rdx
je     c17d <py2_div_mod+0x1cd>
cmp    0x1df(%rip),%rsi        # c1c0 <py2_div_mod+0x210>
sete   %al
cmp    $0xffffffffffffffff,%rdx
mov    %rdx,%r13
sete   %cl
test   %al,%cl
jne    c166 <py2_div_mod+0x1b6>
mov    %rsi,%rax
mov    %rsi,%rdi
cqto
mov    %r13,%rcx
test   %rcx,%rcx
je     c1be <py2_div_mod+0x20e>
mov    %rdi,%rax
idiv   %rcx
mov    %rax,%r15
mov    %rdi,%rax
cqto
cmp    $0xffffffffffffffff,%rcx
jne    c02c <py2_div_mod+0x7c>
mov    $0x0,%edx
jmp    c02f <py2_div_mod+0x7f>
idiv   %rcx
mov    %rdx,0x10(%rsp)
test   %rcx,%rcx
je     c12a <py2_div_mod+0x17a>
cmp    $0xffffffffffffffff,%rcx
mov    %rcx,%r12
cmove  0x17c(%rip),%r12        # c1c8 <py2_div_mod+0x218>
mov    %rdi,%rax
mov    %rcx,%r13
cqto
cmp    $0xffffffffffffffff,%r12
jne    c068 <py2_div_mod+0xb8>
mov    $0x0,%edx
jmp    c06b <py2_div_mod+0xbb>
idiv   %r12
mov    %rdx,0x18(%rsp)
lea    0x40267(%rip),%rdi        # 4c2de <.Ldata5>
mov    $0x4,%esi
mov    $0x2,%edx
lea    0x3438(%rip),%r8        # f4c0 <pt_seq_new>
call   *%r8
mov    %rax,%r14
xor    %rbx,%rbx
mov    0x10(%rsp),%rdx
test   %rdx,%rdx
setne  %sil
mov    %r13,%rax
xor    %rax,%rdx
shr    $0x3f,%rdx
and    %edx,%esi
movzbq %sil,%rsi
mov    %r15,%rax
mov    %rax,%rdi
sub    %rsi,%rdi
lea    0x34d3(%rip),%r8        # f590 <pt_seq_push>
mov    %rdi,%rsi
mov    %r14,%rdi
call   *%r8
mov    0x18(%rsp),%rdx
test   %rdx,%rdx
setne  %dil
mov    %rdx,%rsi
xor    %r12,%rsi
shr    $0x3f,%rsi
test   %dil,%sil
cmovne %r12,%rbx
lea    (%rdx,%rbx,1),%rsi
lea    0x34a2(%rip),%r8        # f590 <pt_seq_push>
mov    %r14,%rdi
call   *%r8
mov    %r14,%rax
jmp    c108 <py2_div_mod+0x158>
mov    %rax,%rcx
mov    %rax,%r14
xor    %rcx,%rax
mov    %rax,%r14
mov    0x20(%rsp),%rbx
mov    0x28(%rsp),%r12
mov    0x30(%rsp),%r13
mov    0x38(%rsp),%r14
mov    0x40(%rsp),%r15
add    $0x50,%rsp
mov    %rbp,%rsp
pop    %rbp
ret
mov    $0x2,%r10d
lea    0x402b3(%rip),%rsi        # 4c3ea <.Ldata8>
mov    $0x22,%edx
lea    0x403c2(%rip),%rcx        # 4c505 <.Ldatac>
mov    $0x3a,%r8d
lea    0x403ef(%rip),%r9        # 4c53f <.Ldatad>
movzbq %r10b,%rdi
mov    %r8,(%rsp)
lea    0x5ec1(%rip),%rax        # 12020 <pt_raise_msg>
call   *%rax
jmp    c0fc <py2_div_mod+0x14c>
lea    0x4030c(%rip),%rdi        # 4c479 <.Ldatab>
mov    $0x8c,%esi
lea    0x56e7(%rip),%rax        # 11860 <pt_fail>
call   *%rax
ud2
mov    $0x2,%eax
lea    0x40261(%rip),%rsi        # 4c3ea <.Ldata8>
mov    $0x22,%edx
lea    0x40277(%rip),%rcx        # 4c40c <.Ldata9>
mov    $0x3a,%r8d
lea    0x402a4(%rip),%r9        # 4c446 <.Ldataa>
mov    $0x33,%r10d
movzbq %al,%rdi
mov    %r10,(%rsp)
lea    0x5e69(%rip),%rax        # 12020 <pt_raise_msg>
call   *%rax
jmp    c0fc <py2_div_mod+0x14c>
ud2
add    %al,(%rax)
add    %al,(%rax)
add    %al,(%rax)
add    %al,0x1(%rax)
add    %al,(%rax)
	...
