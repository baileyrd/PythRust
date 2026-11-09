000000000000bad0 <pythrust_main>:
push   %rbp
mov    %rsp,%rbp
sub    $0x30,%rsp
mov    %rbx,(%rsp)
mov    %r12,0x8(%rsp)
mov    %r13,0x10(%rsp)
mov    %r14,0x18(%rsp)
mov    %r15,0x20(%rsp)
xor    %r13,%r13
mov    %r13,%r14
mov    %r13,%r15
cmp    $0xf4240,%r13
jl     bb0e <pythrust_main+0x3e>
mov    %r15,%rdi
jmp    bb97 <pythrust_main+0xc7>
mov    %rdi,%rbx
mov    0x8(%rbx),%r12
cmp    $0x3e8,%r12
jge    bc48 <pythrust_main+0x178>
lea    0x1(%r12),%rax
mov    %rax,0x8(%rbx)
mov    %r13,%rsi
mov    %rbx,%rdi
call   bda7 <pt_scalar_pair_1>
mov    %r12,0x8(%rbx)
movzbq (%rbx),%rcx
test   %cl,%cl
jne    bc3d <pythrust_main+0x16d>
mov    %r15,%rdi
add    %rax,%rdi
seto   %al
test   %al,%al
jne    bc25 <pythrust_main+0x155>
mov    %r14,%rax
add    %rdx,%rax
mov    %rax,%r14
seto   %al
test   %al,%al
jne    bc0d <pythrust_main+0x13d>
mov    %r13,%rax
add    $0x1,%rax
seto   %al
test   %al,%al
jne    bb97 <pythrust_main+0xc7>
add    $0x1,%r13
seto   %al
test   %al,%al
jne    bbf6 <pythrust_main+0x126>
mov    %rdi,%r15
mov    %rbx,%rdi
jmp    baf9 <pythrust_main+0x29>
lea    0x5d92(%rip),%rdx        # 11930 <pt_write_int>
call   *%rdx
mov    $0x20,%esi
movzbq %sil,%rdi
lea    0x5d50(%rip),%rsi        # 11900 <pt_write_byte>
call   *%rsi
lea    0x5d77(%rip),%rsi        # 11930 <pt_write_int>
mov    %r14,%rdi
call   *%rsi
mov    $0xa,%esi
movzbq %sil,%rdi
lea    0x5d32(%rip),%rsi        # 11900 <pt_write_byte>
call   *%rsi
mov    (%rsp),%rbx
mov    0x8(%rsp),%r12
mov    0x10(%rsp),%r13
mov    0x18(%rsp),%r14
mov    0x20(%rsp),%r15
add    $0x30,%rsp
mov    %rbp,%rsp
pop    %rbp
ret
jmp    bc3d <pythrust_main+0x16d>
lea    0x4049b(%rip),%rdi        # 4c098 <.Ldata1>
mov    $0x8e,%esi
lea    0x5c77(%rip),%rax        # 11880 <pt_fail>
call   *%rax
ud2
lea    0x40594(%rip),%rdi        # 4c1a8 <.Ldata3>
mov    $0x82,%esi
lea    0x5c60(%rip),%r8        # 11880 <pt_fail>
call   *%r8
ud2
lea    0x404fa(%rip),%rdi        # 4c126 <.Ldata2>
mov    $0x82,%esi
lea    0x5c48(%rip),%r10        # 11880 <pt_fail>
call   *%r10
ud2
lea    0x64ec(%rip),%rax        # 12130 <pt_uncaught>
call   *%rax
ud2
lea    0x403b1(%rip),%rdi        # 4c000 <.Ldata0>
mov    $0x98,%esi
lea    0x5c25(%rip),%rax        # 11880 <pt_fail>
call   *%rax
ud2
000000000000bc5f <py1_digits>:
push   %rbp
mov    %rsp,%rbp
sub    $0x20,%rsp
mov    %rbx,(%rsp)
mov    %r12,0x8(%rsp)
mov    %r13,0x10(%rsp)
mov    %r14,0x18(%rsp)
xor    %rax,%rax
mov    %rax,%r13
mov    %r13,%r14
test   %rsi,%rsi
jg     bccf <py1_digits+0x70>
lea    0x40641(%rip),%rdi        # 4c2d4 <.Ldata5>
mov    $0x4,%esi
mov    $0x2,%edx
lea    0x383c(%rip),%rax        # f4e0 <pt_seq_new>
call   *%rax
mov    %rax,%rbx
lea    0x3900(%rip),%rax        # f5b0 <pt_seq_push>
mov    %r13,%rsi
mov    %rbx,%rdi
call   *%rax
lea    0x38f1(%rip),%rax        # f5b0 <pt_seq_push>
mov    %r14,%rsi
mov    %rbx,%rdi
call   *%rax
mov    %rbx,%rax
jmp    bd46 <py1_digits+0xe7>
mov    %rdi,%r12
mov    0x8(%r12),%rbx
cmp    $0x3e8,%rbx
jge    bd90 <py1_digits+0x131>
lea    0x1(%rbx),%rcx
mov    %rcx,0x8(%r12)
mov    $0xa,%edx
mov    %r12,%rdi
call   c0e0 <pt_scalar_pair_2>
mov    %rax,%rdi
mov    %rbx,0x8(%r12)
movzbq (%r12),%rax
test   %al,%al
jne    bd43 <py1_digits+0xe4>
mov    %r13,%rsi
add    $0x1,%rsi
seto   %al
test   %al,%al
jne    bd79 <py1_digits+0x11a>
mov    %r14,%rcx
add    %rdx,%rcx
seto   %al
test   %al,%al
jne    bd62 <py1_digits+0x103>
mov    %rcx,%r14
mov    %rsi,%r13
mov    %rdi,%rsi
mov    %r12,%rdi
jmp    bc83 <py1_digits+0x24>
xor    %rax,%rax
mov    (%rsp),%rbx
mov    0x8(%rsp),%r12
mov    0x10(%rsp),%r13
mov    0x18(%rsp),%r14
add    $0x20,%rsp
mov    %rbp,%rsp
pop    %rbp
ret
lea    0x405f1(%rip),%rdi        # 4c35a <.Ldata7>
mov    $0x82,%esi
lea    0x5b0b(%rip),%rax        # 11880 <pt_fail>
call   *%rax
ud2
lea    0x40558(%rip),%rdi        # 4c2d8 <.Ldata6>
mov    $0x82,%esi
lea    0x5af4(%rip),%rax        # 11880 <pt_fail>
call   *%rax
ud2
lea    0x40493(%rip),%rdi        # 4c22a <.Ldata4>
mov    $0xaa,%esi
lea    0x5add(%rip),%rax        # 11880 <pt_fail>
call   *%rax
ud2
000000000000bda7 <pt_scalar_pair_1>:
push   %rbp
mov    %rsp,%rbp
sub    $0x20,%rsp
mov    %rbx,(%rsp)
mov    %r12,0x8(%rsp)
mov    %r13,0x10(%rsp)
mov    %r14,0x18(%rsp)
xor    %rax,%rax
mov    %rax,%r14
mov    %r14,%r13
test   %rsi,%rsi
jg     bddf <pt_scalar_pair_1+0x38>
mov    %r14,%rax
mov    %r13,%rdx
jmp    be56 <pt_scalar_pair_1+0xaf>
mov    %rdi,%rbx
mov    0x8(%rbx),%r12
cmp    $0x3e8,%r12
jge    bea0 <pt_scalar_pair_1+0xf9>
lea    0x1(%r12),%rcx
mov    %rcx,0x8(%rbx)
mov    $0xa,%edx
mov    %rbx,%rdi
call   c0e0 <pt_scalar_pair_2>
mov    %rax,%rsi
mov    %r12,0x8(%rbx)
movzbq (%rbx),%rax
test   %al,%al
jne    be50 <pt_scalar_pair_1+0xa9>
mov    %r14,%rax
add    $0x1,%rax
seto   %cl
test   %cl,%cl
jne    be89 <pt_scalar_pair_1+0xe2>
mov    %rdx,%rcx
mov    %r13,%rdx
add    %rcx,%rdx
seto   %cl
test   %cl,%cl
jne    be72 <pt_scalar_pair_1+0xcb>
mov    %rbx,%rdi
mov    %rdx,%r13
mov    %rax,%r14
jmp    bdcb <pt_scalar_pair_1+0x24>
xor    %rax,%rax
mov    %rax,%rdx
mov    (%rsp),%rbx
mov    0x8(%rsp),%r12
mov    0x10(%rsp),%r13
mov    0x18(%rsp),%r14
add    $0x20,%rsp
mov    %rbp,%rsp
pop    %rbp
ret
lea    0x404e1(%rip),%rdi        # 4c35a <.Ldata7>
mov    $0x82,%esi
lea    0x59fb(%rip),%rax        # 11880 <pt_fail>
call   *%rax
ud2
lea    0x40448(%rip),%rdi        # 4c2d8 <.Ldata6>
mov    $0x82,%esi
lea    0x59e4(%rip),%rax        # 11880 <pt_fail>
call   *%rax
ud2
lea    0x40383(%rip),%rdi        # 4c22a <.Ldata4>
mov    $0xaa,%esi
lea    0x59cd(%rip),%rax        # 11880 <pt_fail>
call   *%rax
ud2
	...
000000000000beb8 <py2_div_mod>:
push   %rbp
mov    %rsp,%rbp
sub    $0x50,%rsp
mov    %rbx,0x20(%rsp)
mov    %r12,0x28(%rsp)
mov    %r13,0x30(%rsp)
mov    %r14,0x38(%rsp)
mov    %r15,0x40(%rsp)
test   %rdx,%rdx
je     c08a <py2_div_mod+0x1d2>
cmp    0x1e7(%rip),%rsi        # c0d0 <py2_div_mod+0x218>
sete   %al
cmp    $0xffffffffffffffff,%rdx
mov    %rdx,%r13
sete   %cl
test   %al,%cl
jne    c073 <py2_div_mod+0x1bb>
mov    %rsi,%rax
mov    %rsi,%r8
cqto
mov    %r13,%rcx
test   %rcx,%rcx
je     c0cb <py2_div_mod+0x213>
mov    %r8,%rax
idiv   %rcx
mov    %rax,%r15
mov    %r8,%rax
cqto
cmp    $0xffffffffffffffff,%rcx
jne    bf34 <py2_div_mod+0x7c>
mov    $0x0,%edx
jmp    bf37 <py2_div_mod+0x7f>
idiv   %rcx
mov    %rdx,0x10(%rsp)
test   %rcx,%rcx
je     c032 <py2_div_mod+0x17a>
cmp    $0xffffffffffffffff,%rcx
mov    %rcx,%r12
cmove  0x184(%rip),%r12        # c0d8 <py2_div_mod+0x220>
mov    %r8,%rax
mov    %rcx,%r13
cqto
cmp    $0xffffffffffffffff,%r12
jne    bf70 <py2_div_mod+0xb8>
mov    $0x0,%edx
jmp    bf73 <py2_div_mod+0xbb>
idiv   %r12
mov    %rdx,0x18(%rsp)
lea    0x40355(%rip),%rdi        # 4c2d4 <.Ldata5>
mov    $0x4,%esi
mov    $0x2,%edx
lea    0x3550(%rip),%r8        # f4e0 <pt_seq_new>
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
lea    0x35eb(%rip),%r8        # f5b0 <pt_seq_push>
mov    %rdi,%rsi
mov    %r14,%rdi
call   *%r8
mov    0x18(%rsp),%rdx
test   %rdx,%rdx
setne  %r8b
mov    %rdx,%rdi
xor    %r12,%rdi
shr    $0x3f,%rdi
test   %r8b,%dil
cmovne %r12,%rbx
lea    (%rdx,%rbx,1),%rsi
lea    0x35ba(%rip),%r9        # f5b0 <pt_seq_push>
mov    %r14,%rdi
call   *%r9
mov    %r14,%rax
jmp    c010 <py2_div_mod+0x158>
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
mov    $0x2,%r11d
lea    0x4039d(%rip),%rsi        # 4c3dc <.Ldata8>
mov    $0x22,%edx
lea    0x404a8(%rip),%rcx        # 4c4f3 <.Ldatac>
mov    $0x38,%r8d
lea    0x404d3(%rip),%r9        # 4c52b <.Ldatad>
mov    $0x3a,%eax
movzbq %r11b,%rdi
mov    %rax,(%rsp)
lea    0x5fd4(%rip),%rax        # 12040 <pt_raise_msg>
call   *%rax
jmp    c004 <py2_div_mod+0x14c>
lea    0x403ef(%rip),%rdi        # 4c469 <.Ldatab>
mov    $0x8a,%esi
lea    0x57fa(%rip),%rax        # 11880 <pt_fail>
call   *%rax
ud2
mov    $0x2,%eax
lea    0x40346(%rip),%rsi        # 4c3dc <.Ldata8>
mov    $0x22,%edx
lea    0x4035c(%rip),%rcx        # 4c3fe <.Ldata9>
mov    $0x38,%r8d
lea    0x40387(%rip),%r9        # 4c436 <.Ldataa>
mov    $0x33,%r10d
movzbq %al,%rdi
mov    %r10,(%rsp)
lea    0x5f7c(%rip),%rax        # 12040 <pt_raise_msg>
call   *%rax
jmp    c004 <py2_div_mod+0x14c>
ud2
	...
add    %al,(%rax)
addb   $0x0,(%rcx)
add    %al,(%rax)
add    %al,(%rax)
	...
000000000000c0e0 <pt_scalar_pair_2>:
push   %rbp
mov    %rsp,%rbp
sub    $0x10,%rsp
test   %rdx,%rdx
je     c22c <pt_scalar_pair_2+0x14c>
cmp    0x180(%rip),%rsi        # c278 <pt_scalar_pair_2+0x198>
sete   %al
cmp    $0xffffffffffffffff,%rdx
mov    %rdx,%r10
sete   %cl
test   %al,%cl
jne    c214 <pt_scalar_pair_2+0x134>
mov    %rsi,%rax
mov    %rsi,%r9
cqto
mov    %r10,%rcx
test   %rcx,%rcx
je     c26f <pt_scalar_pair_2+0x18f>
mov    %r9,%rax
idiv   %rcx
mov    %rax,%r8
mov    %r9,%rax
cqto
cmp    $0xffffffffffffffff,%rcx
jne    c143 <pt_scalar_pair_2+0x63>
mov    $0x0,%edx
jmp    c146 <pt_scalar_pair_2+0x66>
idiv   %rcx
mov    %rdx,%rdi
test   %r10,%r10
je     c1d2 <pt_scalar_pair_2+0xf2>
cmp    $0xffffffffffffffff,%r10
mov    %r10,%rsi
cmove  0x11f(%rip),%rsi        # c280 <pt_scalar_pair_2+0x1a0>
mov    %r9,%rax
cqto
cmp    $0xffffffffffffffff,%rsi
jne    c17a <pt_scalar_pair_2+0x9a>
mov    $0x0,%edx
jmp    c17d <pt_scalar_pair_2+0x9d>
idiv   %rsi
mov    %rdx,%r9
xor    %rcx,%rcx
mov    %rdi,%rax
test   %rax,%rax
setne  %dil
xor    %r10,%rax
shr    $0x3f,%rax
and    %eax,%edi
movzbq %dil,%rdx
mov    %r8,%rax
sub    %rdx,%rax
mov    %r9,%rdx
test   %rdx,%rdx
setne  %dil
mov    %rdx,%r8
xor    %rsi,%r8
shr    $0x3f,%r8
test   %dil,%r8b
cmovne %rsi,%rcx
add    %rcx,%rdx
jmp    c1c9 <pt_scalar_pair_2+0xe9>
xor    %rax,%rax
mov    %rax,%rdx
add    $0x10,%rsp
mov    %rbp,%rsp
pop    %rbp
ret
mov    $0x2,%edi
lea    0x401fe(%rip),%rsi        # 4c3dc <.Ldata8>
mov    $0x22,%edx
lea    0x40309(%rip),%rcx        # 4c4f3 <.Ldatac>
mov    $0x38,%r8d
lea    0x40334(%rip),%r9        # 4c52b <.Ldatad>
mov    $0x3a,%r10d
movzbq %dil,%rdi
mov    %r10,(%rsp)
lea    0x5e34(%rip),%r10        # 12040 <pt_raise_msg>
call   *%r10
jmp    c1c3 <pt_scalar_pair_2+0xe3>
lea    0x4024e(%rip),%rdi        # 4c469 <.Ldatab>
mov    $0x8a,%esi
lea    0x5659(%rip),%r8        # 11880 <pt_fail>
call   *%r8
ud2
mov    $0x2,%r10d
lea    0x401a3(%rip),%rsi        # 4c3dc <.Ldata8>
mov    $0x22,%edx
lea    0x401b9(%rip),%rcx        # 4c3fe <.Ldata9>
mov    $0x38,%r8d
lea    0x401e4(%rip),%r9        # 4c436 <.Ldataa>
mov    $0x33,%r11d
movzbq %r10b,%rdi
mov    %r11,(%rsp)
lea    0x5dd9(%rip),%r11        # 12040 <pt_raise_msg>
call   *%r11
jmp    c1c3 <pt_scalar_pair_2+0xe3>
ud2
	...
add    %al,(%rax)
addb   $0x0,(%rcx)
add    %al,(%rax)
add    %al,(%rax)
add    %al,(%rax)
nopl   0x0(%rax,%rax,1)
