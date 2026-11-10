
/workspace/card815-run/baseline/float_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000b9c0 <pythrust_main>:
    b9c0:	55                   	push   %rbp
    b9c1:	48 89 e5             	mov    %rsp,%rbp
    b9c4:	48 83 ec 30          	sub    $0x30,%rsp
    b9c8:	48 89 5c 24 10       	mov    %rbx,0x10(%rsp)
    b9cd:	4c 89 64 24 18       	mov    %r12,0x18(%rsp)
    b9d2:	4c 89 6c 24 20       	mov    %r13,0x20(%rsp)
    b9d7:	4c 89 74 24 28       	mov    %r14,0x28(%rsp)
    b9dc:	48 89 fb             	mov    %rdi,%rbx
    b9df:	48 8d 3d 1a 36 04 00 	lea    0x4361a(%rip),%rdi        # 4f000 <.Ldata0>
    b9e6:	be 02 00 00 00       	mov    $0x2,%esi
    b9eb:	ba 01 00 00 00       	mov    $0x1,%edx
    b9f0:	48 8d 05 29 41 00 00 	lea    0x4129(%rip),%rax        # fb20 <pt_seq_new>
    b9f7:	ff d0                	call   *%rax
    b9f9:	49 89 c5             	mov    %rax,%r13
    b9fc:	48 b8 00 00 00 00 00 	movabs $0x401c000000000000,%rax
    ba03:	00 1c 40 
    ba06:	66 48 0f 6e c0       	movq   %rax,%xmm0
    ba0b:	66 48 0f 7e c6       	movq   %xmm0,%rsi
    ba10:	48 8d 05 d9 41 00 00 	lea    0x41d9(%rip),%rax        # fbf0 <pt_seq_push>
    ba17:	4c 89 ef             	mov    %r13,%rdi
    ba1a:	ff d0                	call   *%rax
    ba1c:	48 8d 05 6d 35 00 00 	lea    0x356d(%rip),%rax        # ef90 <pt_list_copy>
    ba23:	4c 89 ef             	mov    %r13,%rdi
    ba26:	ff d0                	call   *%rax
    ba28:	49 89 c6             	mov    %rax,%r14
    ba2b:	be 10 00 00 00       	mov    $0x10,%esi
    ba30:	48 8d 05 b9 37 00 00 	lea    0x37b9(%rip),%rax        # f1f0 <pt_list_scale>
    ba37:	4c 89 f7             	mov    %r14,%rdi
    ba3a:	ff d0                	call   *%rax
    ba3c:	84 c0                	test   %al,%al
    ba3e:	0f 85 bd 00 00 00    	jne    bb01 <pythrust_main+0x141>
    ba44:	4c 8b 63 08          	mov    0x8(%rbx),%r12
    ba48:	49 81 fc e8 03 00 00 	cmp    $0x3e8,%r12
    ba4f:	0f 8d 95 00 00 00    	jge    baea <pythrust_main+0x12a>
    ba55:	49 8d 44 24 01       	lea    0x1(%r12),%rax
    ba5a:	48 89 43 08          	mov    %rax,0x8(%rbx)
    ba5e:	4c 89 f6             	mov    %r14,%rsi
    ba61:	48 89 df             	mov    %rbx,%rdi
    ba64:	e8 af 00 00 00       	call   bb18 <py1_access>
    ba69:	f3 0f 7f 04 24       	movdqu %xmm0,(%rsp)
    ba6e:	4c 89 63 08          	mov    %r12,0x8(%rbx)
    ba72:	48 8d 05 27 3f 00 00 	lea    0x3f27(%rip),%rax        # f9a0 <pt_seq_dec>
    ba79:	4c 89 f7             	mov    %r14,%rdi
    ba7c:	ff d0                	call   *%rax
    ba7e:	48 0f b6 03          	movzbq (%rbx),%rax
    ba82:	84 c0                	test   %al,%al
    ba84:	0f 85 49 00 00 00    	jne    bad3 <pythrust_main+0x113>
    ba8a:	48 8d 05 ef 5a 00 00 	lea    0x5aef(%rip),%rax        # 11580 <pt_write_float>
    ba91:	f3 0f 6f 04 24       	movdqu (%rsp),%xmm0
    ba96:	ff d0                	call   *%rax
    ba98:	b8 0a 00 00 00       	mov    $0xa,%eax
    ba9d:	48 0f b6 f8          	movzbq %al,%rdi
    baa1:	48 8d 05 d8 6a 00 00 	lea    0x6ad8(%rip),%rax        # 12580 <pt_write_byte>
    baa8:	ff d0                	call   *%rax
    baaa:	48 8d 05 ef 3e 00 00 	lea    0x3eef(%rip),%rax        # f9a0 <pt_seq_dec>
    bab1:	4c 89 ef             	mov    %r13,%rdi
    bab4:	ff d0                	call   *%rax
    bab6:	48 8b 5c 24 10       	mov    0x10(%rsp),%rbx
    babb:	4c 8b 64 24 18       	mov    0x18(%rsp),%r12
    bac0:	4c 8b 6c 24 20       	mov    0x20(%rsp),%r13
    bac5:	4c 8b 74 24 28       	mov    0x28(%rsp),%r14
    baca:	48 83 c4 30          	add    $0x30,%rsp
    bace:	48 89 ec             	mov    %rbp,%rsp
    bad1:	5d                   	pop    %rbp
    bad2:	c3                   	ret
    bad3:	4c 89 ef             	mov    %r13,%rdi
    bad6:	48 8d 05 c3 3e 00 00 	lea    0x3ec3(%rip),%rax        # f9a0 <pt_seq_dec>
    badd:	ff d0                	call   *%rax
    badf:	48 8d 05 8a 72 00 00 	lea    0x728a(%rip),%rax        # 12d70 <pt_uncaught>
    bae6:	ff d0                	call   *%rax
    bae8:	0f 0b                	ud2
    baea:	48 8d 3d b8 35 04 00 	lea    0x435b8(%rip),%rdi        # 4f0a9 <.Ldata2>
    baf1:	be ac 00 00 00       	mov    $0xac,%esi
    baf6:	48 8d 05 23 6a 00 00 	lea    0x6a23(%rip),%rax        # 12520 <pt_fail>
    bafd:	ff d0                	call   *%rax
    baff:	0f 0b                	ud2
    bb01:	48 8d 3d fa 34 04 00 	lea    0x434fa(%rip),%rdi        # 4f002 <.Ldata1>
    bb08:	be a7 00 00 00       	mov    $0xa7,%esi
    bb0d:	48 8d 05 0c 6a 00 00 	lea    0x6a0c(%rip),%rax        # 12520 <pt_fail>
    bb14:	ff d0                	call   *%rax
    bb16:	0f 0b                	ud2

Disassembly of section .fini:

/workspace/card815-run/baseline/float_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000fa80 <pt_seq_get>:
    fa80:	41 56                	push   %r14
    fa82:	53                   	push   %rbx
    fa83:	48 83 ec 18          	sub    $0x18,%rsp
    fa87:	48 89 d3             	mov    %rdx,%rbx
    fa8a:	48 89 f9             	mov    %rdi,%rcx
    fa8d:	48 8b 47 18          	mov    0x18(%rdi),%rax
    fa91:	48 85 c0             	test   %rax,%rax
    fa94:	75 48                	jne    fade <pt_seq_get+0x5e>
    fa96:	48 8b 41 30          	mov    0x30(%rcx),%rax
    fa9a:	48 89 f2             	mov    %rsi,%rdx
    fa9d:	48 c1 fa 3f          	sar    $0x3f,%rdx
    faa1:	48 21 c2             	and    %rax,%rdx
    faa4:	48 01 f2             	add    %rsi,%rdx
    faa7:	40 0f 99 c6          	setns  %sil
    faab:	48 39 c2             	cmp    %rax,%rdx
    faae:	0f 92 c0             	setb   %al
    fab1:	40 20 f0             	and    %sil,%al
    fab4:	3c 01                	cmp    $0x1,%al
    fab6:	75 47                	jne    faff <pt_seq_get+0x7f>
    fab8:	48 8b 41 28          	mov    0x28(%rcx),%rax
    fabc:	4c 8b 34 d0          	mov    (%rax,%rdx,8),%r14
    fac0:	48 8b 39             	mov    (%rcx),%rdi
    fac3:	48 8b 71 08          	mov    0x8(%rcx),%rsi
    fac7:	e8 a4 ed ff ff       	call   e870 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq4slot>
    facc:	48 89 c7             	mov    %rax,%rdi
    facf:	48 89 d6             	mov    %rdx,%rsi
    fad2:	4c 89 f2             	mov    %r14,%rdx
    fad5:	e8 c6 cc ff ff       	call   c7a0 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq3inc>
    fada:	31 c0                	xor    %eax,%eax
    fadc:	eb 26                	jmp    fb04 <pt_seq_get+0x84>
    fade:	48 8d 41 18          	lea    0x18(%rcx),%rax
    fae2:	48 89 e7             	mov    %rsp,%rdi
    fae5:	48 89 f2             	mov    %rsi,%rdx
    fae8:	48 89 c6             	mov    %rax,%rsi
    faeb:	e8 70 65 ff ff       	call   6060 <_RINvMs1_NtCsk1kE6r7WBHz_11pythrust_rt4syncINtB6_4SlotINtNtCs40k4W9msRzi_5alloc3vec3VecyEE12get_borrowedNCNvNtB8_3seq10pt_seq_get0EB8_>
    faf0:	83 3c 24 01          	cmpl   $0x1,(%rsp)
    faf4:	75 09                	jne    faff <pt_seq_get+0x7f>
    faf6:	4c 8b 74 24 10       	mov    0x10(%rsp),%r14
    fafb:	31 c0                	xor    %eax,%eax
    fafd:	eb 05                	jmp    fb04 <pt_seq_get+0x84>
    faff:	b0 01                	mov    $0x1,%al
    fb01:	45 31 f6             	xor    %r14d,%r14d
    fb04:	88 03                	mov    %al,(%rbx)
    fb06:	4c 89 f0             	mov    %r14,%rax
    fb09:	48 83 c4 18          	add    $0x18,%rsp
    fb0d:	5b                   	pop    %rbx
    fb0e:	41 5e                	pop    %r14
    fb10:	c3                   	ret
    fb11:	ff 15 71 3d 05 00    	call   *0x53d71(%rip)        # 63888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card815-run/baseline/float_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000f760 <pt_list_set>:
    f760:	41 57                	push   %r15
    f762:	41 56                	push   %r14
    f764:	53                   	push   %rbx
    f765:	48 83 ec 40          	sub    $0x40,%rsp
    f769:	48 89 54 24 20       	mov    %rdx,0x20(%rsp)
    f76e:	48 8b 07             	mov    (%rdi),%rax
    f771:	48 8b 4f 08          	mov    0x8(%rdi),%rcx
    f775:	48 85 c9             	test   %rcx,%rcx
    f778:	74 2d                	je     f7a7 <pt_list_set+0x47>
    f77a:	44 0f b6 00          	movzbl (%rax),%r8d
    f77e:	41 83 c0 9c          	add    $0xffffff9c,%r8d
    f782:	41 83 f8 14          	cmp    $0x14,%r8d
    f786:	77 1f                	ja     f7a7 <pt_list_set+0x47>
    f788:	4c 8d 0d d1 00 04 00 	lea    0x400d1(%rip),%r9        # 4f860 <.Ldatad+0x429>
    f78f:	4f 63 04 81          	movslq (%r9,%r8,4),%r8
    f793:	4d 01 c8             	add    %r9,%r8
    f796:	41 ff e0             	jmp    *%r8
    f799:	48 ff c9             	dec    %rcx
    f79c:	48 ff c0             	inc    %rax
    f79f:	48 89 cb             	mov    %rcx,%rbx
    f7a2:	49 89 c6             	mov    %rax,%r14
    f7a5:	eb 18                	jmp    f7bf <pt_list_set+0x5f>
    f7a7:	48 83 c0 02          	add    $0x2,%rax
    f7ab:	31 db                	xor    %ebx,%ebx
    f7ad:	48 83 e9 02          	sub    $0x2,%rcx
    f7b1:	41 be 01 00 00 00    	mov    $0x1,%r14d
    f7b7:	4c 0f 43 f0          	cmovae %rax,%r14
    f7bb:	48 0f 43 d9          	cmovae %rcx,%rbx
    f7bf:	4c 89 34 24          	mov    %r14,(%rsp)
    f7c3:	48 89 5c 24 08       	mov    %rbx,0x8(%rsp)
    f7c8:	48 8d 44 24 20       	lea    0x20(%rsp),%rax
    f7cd:	48 89 44 24 10       	mov    %rax,0x10(%rsp)
    f7d2:	48 8b 47 18          	mov    0x18(%rdi),%rax
    f7d6:	48 85 c0             	test   %rax,%rax
    f7d9:	0f 85 57 01 00 00    	jne    f936 <pt_list_set+0x1d6>
    f7df:	48 8b 4f 30          	mov    0x30(%rdi),%rcx
    f7e3:	48 89 f0             	mov    %rsi,%rax
    f7e6:	48 c1 f8 3f          	sar    $0x3f,%rax
    f7ea:	48 21 c8             	and    %rcx,%rax
    f7ed:	48 01 f0             	add    %rsi,%rax
    f7f0:	40 0f 99 c6          	setns  %sil
    f7f4:	48 39 c8             	cmp    %rcx,%rax
    f7f7:	0f 92 c1             	setb   %cl
    f7fa:	40 84 ce             	test   %cl,%sil
    f7fd:	0f 84 b5 00 00 00    	je     f8b8 <pt_list_set+0x158>
    f803:	48 8b 4f 28          	mov    0x28(%rdi),%rcx
    f807:	4c 8b 3c c1          	mov    (%rcx,%rax,8),%r15
    f80b:	48 89 14 c1          	mov    %rdx,(%rcx,%rax,8)
    f80f:	48 8b 54 24 20       	mov    0x20(%rsp),%rdx
    f814:	4c 89 f7             	mov    %r14,%rdi
    f817:	48 89 de             	mov    %rbx,%rsi
    f81a:	e8 81 cf ff ff       	call   c7a0 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq3inc>
    f81f:	48 c7 44 24 28 00 00 	movq   $0x0,0x28(%rsp)
    f826:	00 00 
    f828:	48 c7 44 24 30 08 00 	movq   $0x8,0x30(%rsp)
    f82f:	00 00 
    f831:	48 c7 44 24 38 00 00 	movq   $0x0,0x38(%rsp)
    f838:	00 00 
    f83a:	48 8d 4c 24 28       	lea    0x28(%rsp),%rcx
    f83f:	4c 89 f7             	mov    %r14,%rdi
    f842:	48 89 de             	mov    %rbx,%rsi
    f845:	4c 89 fa             	mov    %r15,%rdx
    f848:	e8 63 f4 ff ff       	call   ecb0 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq8dec_into.llvm.12450914586151300226>
    f84d:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
    f852:	48 89 44 24 10       	mov    %rax,0x10(%rsp)
    f857:	0f 10 44 24 28       	movups 0x28(%rsp),%xmm0
    f85c:	0f 29 04 24          	movaps %xmm0,(%rsp)
    f860:	48 89 e3             	mov    %rsp,%rbx
    f863:	66 66 66 66 2e 0f 1f 	data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
    f86a:	84 00 00 00 00 00 
    f870:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
    f875:	48 85 c0             	test   %rax,%rax
    f878:	74 1d                	je     f897 <pt_list_set+0x137>
    f87a:	48 8d 48 ff          	lea    -0x1(%rax),%rcx
    f87e:	48 89 4c 24 10       	mov    %rcx,0x10(%rsp)
    f883:	48 8b 4c 24 08       	mov    0x8(%rsp),%rcx
    f888:	48 8b 7c c1 f8       	mov    -0x8(%rcx,%rax,8),%rdi
    f88d:	48 89 de             	mov    %rbx,%rsi
    f890:	e8 7b f5 ff ff       	call   ee10 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq8drop_one.llvm.12450914586151300226>
    f895:	eb d9                	jmp    f870 <pt_list_set+0x110>
    f897:	48 8b 34 24          	mov    (%rsp),%rsi
    f89b:	48 85 f6             	test   %rsi,%rsi
    f89e:	74 14                	je     f8b4 <pt_list_set+0x154>
    f8a0:	48 8b 7c 24 08       	mov    0x8(%rsp),%rdi
    f8a5:	48 c1 e6 03          	shl    $0x3,%rsi
    f8a9:	ba 08 00 00 00       	mov    $0x8,%edx
    f8ae:	ff 15 74 46 05 00    	call   *0x54674(%rip)        # 63f28 <_DYNAMIC+0x9c8>
    f8b4:	31 c0                	xor    %eax,%eax
    f8b6:	eb 02                	jmp    f8ba <pt_list_set+0x15a>
    f8b8:	b0 01                	mov    $0x1,%al
    f8ba:	48 83 c4 40          	add    $0x40,%rsp
    f8be:	5b                   	pop    %rbx
    f8bf:	41 5e                	pop    %r14
    f8c1:	41 5f                	pop    %r15
    f8c3:	c3                   	ret
    f8c4:	48 83 c0 02          	add    $0x2,%rax
    f8c8:	45 31 c0             	xor    %r8d,%r8d
    f8cb:	48 83 f9 01          	cmp    $0x1,%rcx
    f8cf:	48 0f 44 c1          	cmove  %rcx,%rax
    f8d3:	48 8d 49 fe          	lea    -0x2(%rcx),%rcx
    f8d7:	49 0f 44 c8          	cmove  %r8,%rcx
    f8db:	48 85 c9             	test   %rcx,%rcx
    f8de:	74 43                	je     f923 <pt_list_set+0x1c3>
    f8e0:	4c 8d 48 01          	lea    0x1(%rax),%r9
    f8e4:	48 8d 59 ff          	lea    -0x1(%rcx),%rbx
    f8e8:	45 31 c0             	xor    %r8d,%r8d
    f8eb:	80 38 45             	cmpb   $0x45,(%rax)
    f8ee:	4d 0f 44 c1          	cmove  %r9,%r8
    f8f2:	eb 2f                	jmp    f923 <pt_list_set+0x1c3>
    f8f4:	4c 8d 35 5b 31 04 00 	lea    0x4315b(%rip),%r14        # 52a56 <anon.b3c5fbecbf30bc3e25184a8a8c95e8a7.9.llvm.12450914586151300226>
    f8fb:	bb 01 00 00 00       	mov    $0x1,%ebx
    f900:	e9 ba fe ff ff       	jmp    f7bf <pt_list_set+0x5f>
    f905:	4c 8d 41 fe          	lea    -0x2(%rcx),%r8
    f909:	48 83 c0 02          	add    $0x2,%rax
    f90d:	31 db                	xor    %ebx,%ebx
    f90f:	48 83 f9 01          	cmp    $0x1,%rcx
    f913:	49 0f 45 d8          	cmovne %r8,%rbx
    f917:	48 0f 44 c1          	cmove  %rcx,%rax
    f91b:	49 89 c6             	mov    %rax,%r14
    f91e:	e9 9c fe ff ff       	jmp    f7bf <pt_list_set+0x5f>
    f923:	4d 85 c0             	test   %r8,%r8
    f926:	48 0f 44 d9          	cmove  %rcx,%rbx
    f92a:	49 0f 45 c0          	cmovne %r8,%rax
    f92e:	49 89 c6             	mov    %rax,%r14
    f931:	e9 89 fe ff ff       	jmp    f7bf <pt_list_set+0x5f>
    f936:	48 83 c7 18          	add    $0x18,%rdi
    f93a:	48 89 e1             	mov    %rsp,%rcx
    f93d:	e8 3e 69 ff ff       	call   6280 <_RINvMs1_NtCsk1kE6r7WBHz_11pythrust_rt4syncINtB6_4SlotINtNtCs40k4W9msRzi_5alloc3vec3VecyEE16replace_borrowedNCNvNtB8_3seq11pt_list_set0EB8_>
    f942:	48 89 c1             	mov    %rax,%rcx
    f945:	b0 01                	mov    $0x1,%al
    f947:	48 83 f9 01          	cmp    $0x1,%rcx
    f94b:	0f 85 69 ff ff ff    	jne    f8ba <pt_list_set+0x15a>
    f951:	49 89 d7             	mov    %rdx,%r15
    f954:	e9 c6 fe ff ff       	jmp    f81f <pt_list_set+0xbf>
    f959:	ff 15 29 3f 05 00    	call   *0x53f29(%rip)        # 63888 <_DYNAMIC+0x328>
    f95f:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
    f964:	e8 97 c3 ff ff       	call   bd00 <_RINvNtCscI6d9CVNmLh_4core3ptr9drop_glueINtNtCs40k4W9msRzi_5alloc3vec3VecyEECsk1kE6r7WBHz_11pythrust_rt.llvm.12450914586151300226>
    f969:	ff 15 19 3f 05 00    	call   *0x53f19(%rip)        # 63888 <_DYNAMIC+0x328>
    f96f:	48 8b 34 24          	mov    (%rsp),%rsi
    f973:	48 85 f6             	test   %rsi,%rsi
    f976:	75 06                	jne    f97e <pt_list_set+0x21e>
    f978:	ff 15 0a 3f 05 00    	call   *0x53f0a(%rip)        # 63888 <_DYNAMIC+0x328>
    f97e:	48 8b 7c 24 08       	mov    0x8(%rsp),%rdi
    f983:	48 c1 e6 03          	shl    $0x3,%rsi
    f987:	ba 08 00 00 00       	mov    $0x8,%edx
    f98c:	ff 15 96 45 05 00    	call   *0x54596(%rip)        # 63f28 <_DYNAMIC+0x9c8>
    f992:	ff 15 f0 3e 05 00    	call   *0x53ef0(%rip)        # 63888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card815-run/baseline/float_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000bb18 <py1_access>:
    bb18:	55                   	push   %rbp
    bb19:	48 89 e5             	mov    %rsp,%rbp
    bb1c:	48 83 ec 60          	sub    $0x60,%rsp
    bb20:	48 89 5c 24 30       	mov    %rbx,0x30(%rsp)
    bb25:	4c 89 64 24 38       	mov    %r12,0x38(%rsp)
    bb2a:	4c 89 6c 24 40       	mov    %r13,0x40(%rsp)
    bb2f:	4c 89 74 24 48       	mov    %r14,0x48(%rsp)
    bb34:	4c 89 7c 24 50       	mov    %r15,0x50(%rsp)
    bb39:	49 89 f7             	mov    %rsi,%r15
    bb3c:	4d 33 f6             	xor    %r14,%r14
    bb3f:	66 0f 57 c0          	xorpd  %xmm0,%xmm0
    bb43:	f3 0f 7f 44 24 18    	movdqu %xmm0,0x18(%rsp)
    bb49:	48 8d 5c 24 10       	lea    0x10(%rsp),%rbx
    bb4e:	48 b9 00 00 00 00 00 	movabs $0x401c000000000000,%rcx
    bb55:	00 1c 40 
    bb58:	66 48 0f 6e c1       	movq   %rcx,%xmm0
    bb5d:	66 49 0f 7e c4       	movq   %xmm0,%r12
    bb62:	49 81 fe 00 09 3d 00 	cmp    $0x3d0900,%r14
    bb69:	0f 8c 0b 00 00 00    	jl     bb7a <py1_access+0x62>
    bb6f:	f3 0f 6f 44 24 18    	movdqu 0x18(%rsp),%xmm0
    bb75:	e9 c3 00 00 00       	jmp    bc3d <py1_access+0x125>
    bb7a:	4c 89 f1             	mov    %r14,%rcx
    bb7d:	48 c1 f9 03          	sar    $0x3,%rcx
    bb81:	48 c1 e9 3c          	shr    $0x3c,%rcx
    bb85:	49 8d 14 0e          	lea    (%r14,%rcx,1),%rdx
    bb89:	48 83 e2 f0          	and    $0xfffffffffffffff0,%rdx
    bb8d:	4c 89 f1             	mov    %r14,%rcx
    bb90:	48 2b ca             	sub    %rdx,%rcx
    bb93:	4c 3b f2             	cmp    %rdx,%r14
    bb96:	0f 95 c2             	setne  %dl
    bb99:	48 89 cf             	mov    %rcx,%rdi
    bb9c:	48 83 f7 10          	xor    $0x10,%rdi
    bba0:	48 33 f6             	xor    %rsi,%rsi
    bba3:	48 c1 ef 3f          	shr    $0x3f,%rdi
    bba7:	40 84 d7             	test   %dl,%dil
    bbaa:	48 0f 45 35 46 01 00 	cmovne 0x146(%rip),%rsi        # bcf8 <py1_access+0x1e0>
    bbb1:	00 
    bbb2:	4c 8d 2c 31          	lea    (%rcx,%rsi,1),%r13
    bbb6:	4c 8d 05 c3 3e 00 00 	lea    0x3ec3(%rip),%r8        # fa80 <pt_seq_get>
    bbbd:	48 89 da             	mov    %rbx,%rdx
    bbc0:	4c 89 ee             	mov    %r13,%rsi
    bbc3:	4c 89 ff             	mov    %r15,%rdi
    bbc6:	41 ff d0             	call   *%r8
    bbc9:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
    bbce:	48 0f b6 74 24 10    	movzbq 0x10(%rsp),%rsi
    bbd4:	40 80 fe 01          	cmp    $0x1,%sil
    bbd8:	0f 84 d9 00 00 00    	je     bcb7 <py1_access+0x19f>
    bbde:	4c 8d 05 7b 3b 00 00 	lea    0x3b7b(%rip),%r8        # f760 <pt_list_set>
    bbe5:	4c 89 ee             	mov    %r13,%rsi
    bbe8:	4c 89 e2             	mov    %r12,%rdx
    bbeb:	4c 89 ff             	mov    %r15,%rdi
    bbee:	41 ff d0             	call   *%r8
    bbf1:	3c 01                	cmp    $0x1,%al
    bbf3:	0f 84 7d 00 00 00    	je     bc76 <py1_access+0x15e>
    bbf9:	4d 89 f2             	mov    %r14,%r10
    bbfc:	49 83 c2 01          	add    $0x1,%r10
    bc00:	0f 90 c1             	seto   %cl
    bc03:	48 8b 44 24 28       	mov    0x28(%rsp),%rax
    bc08:	66 48 0f 6e c8       	movq   %rax,%xmm1
    bc0d:	f3 0f 6f 44 24 18    	movdqu 0x18(%rsp),%xmm0
    bc13:	f2 0f 58 c1          	addsd  %xmm1,%xmm0
    bc17:	84 c9                	test   %cl,%cl
    bc19:	0f 85 1e 00 00 00    	jne    bc3d <py1_access+0x125>
    bc1f:	49 83 c6 01          	add    $0x1,%r14
    bc23:	0f 90 c0             	seto   %al
    bc26:	84 c0                	test   %al,%al
    bc28:	0f 85 31 00 00 00    	jne    bc5f <py1_access+0x147>
    bc2e:	f3 0f 7f 44 24 18    	movdqu %xmm0,0x18(%rsp)
    bc34:	e9 29 ff ff ff       	jmp    bb62 <py1_access+0x4a>
    bc39:	66 0f 57 c0          	xorpd  %xmm0,%xmm0
    bc3d:	48 8b 5c 24 30       	mov    0x30(%rsp),%rbx
    bc42:	4c 8b 64 24 38       	mov    0x38(%rsp),%r12
    bc47:	4c 8b 6c 24 40       	mov    0x40(%rsp),%r13
    bc4c:	4c 8b 74 24 48       	mov    0x48(%rsp),%r14
    bc51:	4c 8b 7c 24 50       	mov    0x50(%rsp),%r15
    bc56:	48 83 c4 60          	add    $0x60,%rsp
    bc5a:	48 89 ec             	mov    %rbp,%rsp
    bc5d:	5d                   	pop    %rbp
    bc5e:	c3                   	ret
    bc5f:	48 8d 3d 06 36 04 00 	lea    0x43606(%rip),%rdi        # 4f26c <.Ldata9>
    bc66:	be a1 00 00 00       	mov    $0xa1,%esi
    bc6b:	48 8d 05 ae 68 00 00 	lea    0x68ae(%rip),%rax        # 12520 <pt_fail>
    bc72:	ff d0                	call   *%rax
    bc74:	0f 0b                	ud2
    bc76:	b8 01 00 00 00       	mov    $0x1,%eax
    bc7b:	48 8d 35 50 37 04 00 	lea    0x43750(%rip),%rsi        # 4f3d2 <.Ldatab>
    bc82:	ba 22 00 00 00       	mov    $0x22,%edx
    bc87:	48 8d 0d 66 37 04 00 	lea    0x43766(%rip),%rcx        # 4f3f4 <.Ldatac>
    bc8e:	41 b8 43 00 00 00    	mov    $0x43,%r8d
    bc94:	4c 8d 0d 9c 37 04 00 	lea    0x4379c(%rip),%r9        # 4f437 <.Ldatad>
    bc9b:	41 ba 27 00 00 00    	mov    $0x27,%r10d
    bca1:	48 0f b6 f8          	movzbq %al,%rdi
    bca5:	4c 89 14 24          	mov    %r10,(%rsp)
    bca9:	48 8d 05 d0 6f 00 00 	lea    0x6fd0(%rip),%rax        # 12c80 <pt_raise_msg>
    bcb0:	ff d0                	call   *%rax
    bcb2:	e9 82 ff ff ff       	jmp    bc39 <py1_access+0x121>
    bcb7:	b8 01 00 00 00       	mov    $0x1,%eax
    bcbc:	48 8d 35 23 35 04 00 	lea    0x43523(%rip),%rsi        # 4f1e6 <.Ldata6>
    bcc3:	ba 17 00 00 00       	mov    $0x17,%edx
    bcc8:	48 8d 0d 2e 35 04 00 	lea    0x4352e(%rip),%rcx        # 4f1fd <.Ldata7>
    bccf:	41 b8 44 00 00 00    	mov    $0x44,%r8d
    bcd5:	4c 8d 0d 65 35 04 00 	lea    0x43565(%rip),%r9        # 4f241 <.Ldata8>
    bcdc:	41 ba 2b 00 00 00    	mov    $0x2b,%r10d
    bce2:	48 0f b6 f8          	movzbq %al,%rdi
    bce6:	4c 89 14 24          	mov    %r10,(%rsp)
    bcea:	48 8d 05 8f 6f 00 00 	lea    0x6f8f(%rip),%rax        # 12c80 <pt_raise_msg>
    bcf1:	ff d0                	call   *%rax
    bcf3:	e9 41 ff ff ff       	jmp    bc39 <py1_access+0x121>
    bcf8:	10 00                	adc    %al,(%rax)
    bcfa:	00 00                	add    %al,(%rax)
    bcfc:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:
