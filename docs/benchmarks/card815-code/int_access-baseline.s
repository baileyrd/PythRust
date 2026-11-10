
/workspace/card815-run/baseline/int_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000b9c0 <pythrust_main>:
    b9c0:	55                   	push   %rbp
    b9c1:	48 89 e5             	mov    %rsp,%rbp
    b9c4:	48 83 ec 30          	sub    $0x30,%rsp
    b9c8:	48 89 1c 24          	mov    %rbx,(%rsp)
    b9cc:	4c 89 64 24 08       	mov    %r12,0x8(%rsp)
    b9d1:	4c 89 6c 24 10       	mov    %r13,0x10(%rsp)
    b9d6:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    b9db:	4c 89 7c 24 20       	mov    %r15,0x20(%rsp)
    b9e0:	48 89 fb             	mov    %rdi,%rbx
    b9e3:	48 8d 3d 16 36 04 00 	lea    0x43616(%rip),%rdi        # 4f000 <.Ldata0>
    b9ea:	be 02 00 00 00       	mov    $0x2,%esi
    b9ef:	ba 01 00 00 00       	mov    $0x1,%edx
    b9f4:	48 8d 05 15 41 00 00 	lea    0x4115(%rip),%rax        # fb10 <pt_seq_new>
    b9fb:	ff d0                	call   *%rax
    b9fd:	49 89 c5             	mov    %rax,%r13
    ba00:	be 07 00 00 00       	mov    $0x7,%esi
    ba05:	48 8d 05 d4 41 00 00 	lea    0x41d4(%rip),%rax        # fbe0 <pt_seq_push>
    ba0c:	4c 89 ef             	mov    %r13,%rdi
    ba0f:	ff d0                	call   *%rax
    ba11:	48 8d 05 68 35 00 00 	lea    0x3568(%rip),%rax        # ef80 <pt_list_copy>
    ba18:	4c 89 ef             	mov    %r13,%rdi
    ba1b:	ff d0                	call   *%rax
    ba1d:	49 89 c7             	mov    %rax,%r15
    ba20:	be 10 00 00 00       	mov    $0x10,%esi
    ba25:	48 8d 0d b4 37 00 00 	lea    0x37b4(%rip),%rcx        # f1e0 <pt_list_scale>
    ba2c:	4c 89 ff             	mov    %r15,%rdi
    ba2f:	ff d1                	call   *%rcx
    ba31:	84 c0                	test   %al,%al
    ba33:	0f 85 bd 00 00 00    	jne    baf6 <pythrust_main+0x136>
    ba39:	4c 8b 63 08          	mov    0x8(%rbx),%r12
    ba3d:	49 81 fc e8 03 00 00 	cmp    $0x3e8,%r12
    ba44:	0f 8d 95 00 00 00    	jge    badf <pythrust_main+0x11f>
    ba4a:	49 8d 44 24 01       	lea    0x1(%r12),%rax
    ba4f:	48 89 43 08          	mov    %rax,0x8(%rbx)
    ba53:	4c 89 fe             	mov    %r15,%rsi
    ba56:	48 89 df             	mov    %rbx,%rdi
    ba59:	e8 b2 00 00 00       	call   bb10 <py1_access>
    ba5e:	49 89 c6             	mov    %rax,%r14
    ba61:	4c 89 63 08          	mov    %r12,0x8(%rbx)
    ba65:	48 8d 05 24 3f 00 00 	lea    0x3f24(%rip),%rax        # f990 <pt_seq_dec>
    ba6c:	4c 89 ff             	mov    %r15,%rdi
    ba6f:	ff d0                	call   *%rax
    ba71:	48 0f b6 03          	movzbq (%rbx),%rax
    ba75:	84 c0                	test   %al,%al
    ba77:	0f 85 4b 00 00 00    	jne    bac8 <pythrust_main+0x108>
    ba7d:	48 8d 05 ac 6a 00 00 	lea    0x6aac(%rip),%rax        # 12530 <pt_write_int>
    ba84:	4c 89 f7             	mov    %r14,%rdi
    ba87:	ff d0                	call   *%rax
    ba89:	b8 0a 00 00 00       	mov    $0xa,%eax
    ba8e:	48 0f b6 f8          	movzbq %al,%rdi
    ba92:	48 8d 05 67 6a 00 00 	lea    0x6a67(%rip),%rax        # 12500 <pt_write_byte>
    ba99:	ff d0                	call   *%rax
    ba9b:	48 8d 05 ee 3e 00 00 	lea    0x3eee(%rip),%rax        # f990 <pt_seq_dec>
    baa2:	4c 89 ef             	mov    %r13,%rdi
    baa5:	ff d0                	call   *%rax
    baa7:	48 8b 1c 24          	mov    (%rsp),%rbx
    baab:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
    bab0:	4c 8b 6c 24 10       	mov    0x10(%rsp),%r13
    bab5:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
    baba:	4c 8b 7c 24 20       	mov    0x20(%rsp),%r15
    babf:	48 83 c4 30          	add    $0x30,%rsp
    bac3:	48 89 ec             	mov    %rbp,%rsp
    bac6:	5d                   	pop    %rbp
    bac7:	c3                   	ret
    bac8:	4c 89 ef             	mov    %r13,%rdi
    bacb:	48 8d 05 be 3e 00 00 	lea    0x3ebe(%rip),%rax        # f990 <pt_seq_dec>
    bad2:	ff d0                	call   *%rax
    bad4:	48 8d 05 a5 73 00 00 	lea    0x73a5(%rip),%rax        # 12e80 <pt_uncaught>
    badb:	ff d0                	call   *%rax
    badd:	0f 0b                	ud2
    badf:	48 8d 3d bd 35 04 00 	lea    0x435bd(%rip),%rdi        # 4f0a3 <.Ldata2>
    bae6:	be a6 00 00 00       	mov    $0xa6,%esi
    baeb:	48 8d 05 ae 69 00 00 	lea    0x69ae(%rip),%rax        # 124a0 <pt_fail>
    baf2:	ff d0                	call   *%rax
    baf4:	0f 0b                	ud2
    baf6:	48 8d 3d 05 35 04 00 	lea    0x43505(%rip),%rdi        # 4f002 <.Ldata1>
    bafd:	be a1 00 00 00       	mov    $0xa1,%esi
    bb02:	48 8d 05 97 69 00 00 	lea    0x6997(%rip),%rax        # 124a0 <pt_fail>
    bb09:	ff d0                	call   *%rax
    bb0b:	0f 0b                	ud2

Disassembly of section .fini:

/workspace/card815-run/baseline/int_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000fa70 <pt_seq_get>:
    fa70:	41 56                	push   %r14
    fa72:	53                   	push   %rbx
    fa73:	48 83 ec 18          	sub    $0x18,%rsp
    fa77:	48 89 d3             	mov    %rdx,%rbx
    fa7a:	48 89 f9             	mov    %rdi,%rcx
    fa7d:	48 8b 47 18          	mov    0x18(%rdi),%rax
    fa81:	48 85 c0             	test   %rax,%rax
    fa84:	75 48                	jne    face <pt_seq_get+0x5e>
    fa86:	48 8b 41 30          	mov    0x30(%rcx),%rax
    fa8a:	48 89 f2             	mov    %rsi,%rdx
    fa8d:	48 c1 fa 3f          	sar    $0x3f,%rdx
    fa91:	48 21 c2             	and    %rax,%rdx
    fa94:	48 01 f2             	add    %rsi,%rdx
    fa97:	40 0f 99 c6          	setns  %sil
    fa9b:	48 39 c2             	cmp    %rax,%rdx
    fa9e:	0f 92 c0             	setb   %al
    faa1:	40 20 f0             	and    %sil,%al
    faa4:	3c 01                	cmp    $0x1,%al
    faa6:	75 47                	jne    faef <pt_seq_get+0x7f>
    faa8:	48 8b 41 28          	mov    0x28(%rcx),%rax
    faac:	4c 8b 34 d0          	mov    (%rax,%rdx,8),%r14
    fab0:	48 8b 39             	mov    (%rcx),%rdi
    fab3:	48 8b 71 08          	mov    0x8(%rcx),%rsi
    fab7:	e8 a4 ed ff ff       	call   e860 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq4slot>
    fabc:	48 89 c7             	mov    %rax,%rdi
    fabf:	48 89 d6             	mov    %rdx,%rsi
    fac2:	4c 89 f2             	mov    %r14,%rdx
    fac5:	e8 c6 cc ff ff       	call   c790 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq3inc>
    faca:	31 c0                	xor    %eax,%eax
    facc:	eb 26                	jmp    faf4 <pt_seq_get+0x84>
    face:	48 8d 41 18          	lea    0x18(%rcx),%rax
    fad2:	48 89 e7             	mov    %rsp,%rdi
    fad5:	48 89 f2             	mov    %rsi,%rdx
    fad8:	48 89 c6             	mov    %rax,%rsi
    fadb:	e8 80 65 ff ff       	call   6060 <_RINvMs1_NtCsk1kE6r7WBHz_11pythrust_rt4syncINtB6_4SlotINtNtCs40k4W9msRzi_5alloc3vec3VecyEE12get_borrowedNCNvNtB8_3seq10pt_seq_get0EB8_>
    fae0:	83 3c 24 01          	cmpl   $0x1,(%rsp)
    fae4:	75 09                	jne    faef <pt_seq_get+0x7f>
    fae6:	4c 8b 74 24 10       	mov    0x10(%rsp),%r14
    faeb:	31 c0                	xor    %eax,%eax
    faed:	eb 05                	jmp    faf4 <pt_seq_get+0x84>
    faef:	b0 01                	mov    $0x1,%al
    faf1:	45 31 f6             	xor    %r14d,%r14d
    faf4:	88 03                	mov    %al,(%rbx)
    faf6:	4c 89 f0             	mov    %r14,%rax
    faf9:	48 83 c4 18          	add    $0x18,%rsp
    fafd:	5b                   	pop    %rbx
    fafe:	41 5e                	pop    %r14
    fb00:	c3                   	ret
    fb01:	ff 15 81 3d 05 00    	call   *0x53d81(%rip)        # 63888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card815-run/baseline/int_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000f750 <pt_list_set>:
    f750:	41 57                	push   %r15
    f752:	41 56                	push   %r14
    f754:	53                   	push   %rbx
    f755:	48 83 ec 40          	sub    $0x40,%rsp
    f759:	48 89 54 24 20       	mov    %rdx,0x20(%rsp)
    f75e:	48 8b 07             	mov    (%rdi),%rax
    f761:	48 8b 4f 08          	mov    0x8(%rdi),%rcx
    f765:	48 85 c9             	test   %rcx,%rcx
    f768:	74 2d                	je     f797 <pt_list_set+0x47>
    f76a:	44 0f b6 00          	movzbl (%rax),%r8d
    f76e:	41 83 c0 9c          	add    $0xffffff9c,%r8d
    f772:	41 83 f8 14          	cmp    $0x14,%r8d
    f776:	77 1f                	ja     f797 <pt_list_set+0x47>
    f778:	4c 8d 0d 51 01 04 00 	lea    0x40151(%rip),%r9        # 4f8d0 <.Ldatae+0x426>
    f77f:	4f 63 04 81          	movslq (%r9,%r8,4),%r8
    f783:	4d 01 c8             	add    %r9,%r8
    f786:	41 ff e0             	jmp    *%r8
    f789:	48 ff c9             	dec    %rcx
    f78c:	48 ff c0             	inc    %rax
    f78f:	48 89 cb             	mov    %rcx,%rbx
    f792:	49 89 c6             	mov    %rax,%r14
    f795:	eb 18                	jmp    f7af <pt_list_set+0x5f>
    f797:	48 83 c0 02          	add    $0x2,%rax
    f79b:	31 db                	xor    %ebx,%ebx
    f79d:	48 83 e9 02          	sub    $0x2,%rcx
    f7a1:	41 be 01 00 00 00    	mov    $0x1,%r14d
    f7a7:	4c 0f 43 f0          	cmovae %rax,%r14
    f7ab:	48 0f 43 d9          	cmovae %rcx,%rbx
    f7af:	4c 89 34 24          	mov    %r14,(%rsp)
    f7b3:	48 89 5c 24 08       	mov    %rbx,0x8(%rsp)
    f7b8:	48 8d 44 24 20       	lea    0x20(%rsp),%rax
    f7bd:	48 89 44 24 10       	mov    %rax,0x10(%rsp)
    f7c2:	48 8b 47 18          	mov    0x18(%rdi),%rax
    f7c6:	48 85 c0             	test   %rax,%rax
    f7c9:	0f 85 57 01 00 00    	jne    f926 <pt_list_set+0x1d6>
    f7cf:	48 8b 4f 30          	mov    0x30(%rdi),%rcx
    f7d3:	48 89 f0             	mov    %rsi,%rax
    f7d6:	48 c1 f8 3f          	sar    $0x3f,%rax
    f7da:	48 21 c8             	and    %rcx,%rax
    f7dd:	48 01 f0             	add    %rsi,%rax
    f7e0:	40 0f 99 c6          	setns  %sil
    f7e4:	48 39 c8             	cmp    %rcx,%rax
    f7e7:	0f 92 c1             	setb   %cl
    f7ea:	40 84 ce             	test   %cl,%sil
    f7ed:	0f 84 b5 00 00 00    	je     f8a8 <pt_list_set+0x158>
    f7f3:	48 8b 4f 28          	mov    0x28(%rdi),%rcx
    f7f7:	4c 8b 3c c1          	mov    (%rcx,%rax,8),%r15
    f7fb:	48 89 14 c1          	mov    %rdx,(%rcx,%rax,8)
    f7ff:	48 8b 54 24 20       	mov    0x20(%rsp),%rdx
    f804:	4c 89 f7             	mov    %r14,%rdi
    f807:	48 89 de             	mov    %rbx,%rsi
    f80a:	e8 81 cf ff ff       	call   c790 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq3inc>
    f80f:	48 c7 44 24 28 00 00 	movq   $0x0,0x28(%rsp)
    f816:	00 00 
    f818:	48 c7 44 24 30 08 00 	movq   $0x8,0x30(%rsp)
    f81f:	00 00 
    f821:	48 c7 44 24 38 00 00 	movq   $0x0,0x38(%rsp)
    f828:	00 00 
    f82a:	48 8d 4c 24 28       	lea    0x28(%rsp),%rcx
    f82f:	4c 89 f7             	mov    %r14,%rdi
    f832:	48 89 de             	mov    %rbx,%rsi
    f835:	4c 89 fa             	mov    %r15,%rdx
    f838:	e8 63 f4 ff ff       	call   eca0 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq8dec_into.llvm.12450914586151300226>
    f83d:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
    f842:	48 89 44 24 10       	mov    %rax,0x10(%rsp)
    f847:	0f 10 44 24 28       	movups 0x28(%rsp),%xmm0
    f84c:	0f 29 04 24          	movaps %xmm0,(%rsp)
    f850:	48 89 e3             	mov    %rsp,%rbx
    f853:	66 66 66 66 2e 0f 1f 	data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
    f85a:	84 00 00 00 00 00 
    f860:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
    f865:	48 85 c0             	test   %rax,%rax
    f868:	74 1d                	je     f887 <pt_list_set+0x137>
    f86a:	48 8d 48 ff          	lea    -0x1(%rax),%rcx
    f86e:	48 89 4c 24 10       	mov    %rcx,0x10(%rsp)
    f873:	48 8b 4c 24 08       	mov    0x8(%rsp),%rcx
    f878:	48 8b 7c c1 f8       	mov    -0x8(%rcx,%rax,8),%rdi
    f87d:	48 89 de             	mov    %rbx,%rsi
    f880:	e8 7b f5 ff ff       	call   ee00 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq8drop_one.llvm.12450914586151300226>
    f885:	eb d9                	jmp    f860 <pt_list_set+0x110>
    f887:	48 8b 34 24          	mov    (%rsp),%rsi
    f88b:	48 85 f6             	test   %rsi,%rsi
    f88e:	74 14                	je     f8a4 <pt_list_set+0x154>
    f890:	48 8b 7c 24 08       	mov    0x8(%rsp),%rdi
    f895:	48 c1 e6 03          	shl    $0x3,%rsi
    f899:	ba 08 00 00 00       	mov    $0x8,%edx
    f89e:	ff 15 84 46 05 00    	call   *0x54684(%rip)        # 63f28 <_DYNAMIC+0x9c8>
    f8a4:	31 c0                	xor    %eax,%eax
    f8a6:	eb 02                	jmp    f8aa <pt_list_set+0x15a>
    f8a8:	b0 01                	mov    $0x1,%al
    f8aa:	48 83 c4 40          	add    $0x40,%rsp
    f8ae:	5b                   	pop    %rbx
    f8af:	41 5e                	pop    %r14
    f8b1:	41 5f                	pop    %r15
    f8b3:	c3                   	ret
    f8b4:	48 83 c0 02          	add    $0x2,%rax
    f8b8:	45 31 c0             	xor    %r8d,%r8d
    f8bb:	48 83 f9 01          	cmp    $0x1,%rcx
    f8bf:	48 0f 44 c1          	cmove  %rcx,%rax
    f8c3:	48 8d 49 fe          	lea    -0x2(%rcx),%rcx
    f8c7:	49 0f 44 c8          	cmove  %r8,%rcx
    f8cb:	48 85 c9             	test   %rcx,%rcx
    f8ce:	74 43                	je     f913 <pt_list_set+0x1c3>
    f8d0:	4c 8d 48 01          	lea    0x1(%rax),%r9
    f8d4:	48 8d 59 ff          	lea    -0x1(%rcx),%rbx
    f8d8:	45 31 c0             	xor    %r8d,%r8d
    f8db:	80 38 45             	cmpb   $0x45,(%rax)
    f8de:	4d 0f 44 c1          	cmove  %r9,%r8
    f8e2:	eb 2f                	jmp    f913 <pt_list_set+0x1c3>
    f8e4:	4c 8d 35 db 31 04 00 	lea    0x431db(%rip),%r14        # 52ac6 <anon.b3c5fbecbf30bc3e25184a8a8c95e8a7.9.llvm.12450914586151300226>
    f8eb:	bb 01 00 00 00       	mov    $0x1,%ebx
    f8f0:	e9 ba fe ff ff       	jmp    f7af <pt_list_set+0x5f>
    f8f5:	4c 8d 41 fe          	lea    -0x2(%rcx),%r8
    f8f9:	48 83 c0 02          	add    $0x2,%rax
    f8fd:	31 db                	xor    %ebx,%ebx
    f8ff:	48 83 f9 01          	cmp    $0x1,%rcx
    f903:	49 0f 45 d8          	cmovne %r8,%rbx
    f907:	48 0f 44 c1          	cmove  %rcx,%rax
    f90b:	49 89 c6             	mov    %rax,%r14
    f90e:	e9 9c fe ff ff       	jmp    f7af <pt_list_set+0x5f>
    f913:	4d 85 c0             	test   %r8,%r8
    f916:	48 0f 44 d9          	cmove  %rcx,%rbx
    f91a:	49 0f 45 c0          	cmovne %r8,%rax
    f91e:	49 89 c6             	mov    %rax,%r14
    f921:	e9 89 fe ff ff       	jmp    f7af <pt_list_set+0x5f>
    f926:	48 83 c7 18          	add    $0x18,%rdi
    f92a:	48 89 e1             	mov    %rsp,%rcx
    f92d:	e8 4e 69 ff ff       	call   6280 <_RINvMs1_NtCsk1kE6r7WBHz_11pythrust_rt4syncINtB6_4SlotINtNtCs40k4W9msRzi_5alloc3vec3VecyEE16replace_borrowedNCNvNtB8_3seq11pt_list_set0EB8_>
    f932:	48 89 c1             	mov    %rax,%rcx
    f935:	b0 01                	mov    $0x1,%al
    f937:	48 83 f9 01          	cmp    $0x1,%rcx
    f93b:	0f 85 69 ff ff ff    	jne    f8aa <pt_list_set+0x15a>
    f941:	49 89 d7             	mov    %rdx,%r15
    f944:	e9 c6 fe ff ff       	jmp    f80f <pt_list_set+0xbf>
    f949:	ff 15 39 3f 05 00    	call   *0x53f39(%rip)        # 63888 <_DYNAMIC+0x328>
    f94f:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
    f954:	e8 97 c3 ff ff       	call   bcf0 <_RINvNtCscI6d9CVNmLh_4core3ptr9drop_glueINtNtCs40k4W9msRzi_5alloc3vec3VecyEECsk1kE6r7WBHz_11pythrust_rt.llvm.12450914586151300226>
    f959:	ff 15 29 3f 05 00    	call   *0x53f29(%rip)        # 63888 <_DYNAMIC+0x328>
    f95f:	48 8b 34 24          	mov    (%rsp),%rsi
    f963:	48 85 f6             	test   %rsi,%rsi
    f966:	75 06                	jne    f96e <pt_list_set+0x21e>
    f968:	ff 15 1a 3f 05 00    	call   *0x53f1a(%rip)        # 63888 <_DYNAMIC+0x328>
    f96e:	48 8b 7c 24 08       	mov    0x8(%rsp),%rdi
    f973:	48 c1 e6 03          	shl    $0x3,%rsi
    f977:	ba 08 00 00 00       	mov    $0x8,%edx
    f97c:	ff 15 a6 45 05 00    	call   *0x545a6(%rip)        # 63f28 <_DYNAMIC+0x9c8>
    f982:	ff 15 00 3f 05 00    	call   *0x53f00(%rip)        # 63888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card815-run/baseline/int_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000bb10 <py1_access>:
    bb10:	55                   	push   %rbp
    bb11:	48 89 e5             	mov    %rsp,%rbp
    bb14:	48 83 ec 50          	sub    $0x50,%rsp
    bb18:	48 89 5c 24 20       	mov    %rbx,0x20(%rsp)
    bb1d:	4c 89 64 24 28       	mov    %r12,0x28(%rsp)
    bb22:	4c 89 6c 24 30       	mov    %r13,0x30(%rsp)
    bb27:	4c 89 74 24 38       	mov    %r14,0x38(%rsp)
    bb2c:	4c 89 7c 24 40       	mov    %r15,0x40(%rsp)
    bb31:	49 89 f6             	mov    %rsi,%r14
    bb34:	48 33 db             	xor    %rbx,%rbx
    bb37:	4c 8d 64 24 10       	lea    0x10(%rsp),%r12
    bb3c:	49 89 df             	mov    %rbx,%r15
    bb3f:	48 81 fb 00 09 3d 00 	cmp    $0x3d0900,%rbx
    bb46:	0f 8d b7 00 00 00    	jge    bc03 <py1_access+0xf3>
    bb4c:	48 89 d8             	mov    %rbx,%rax
    bb4f:	48 c1 f8 03          	sar    $0x3,%rax
    bb53:	48 c1 e8 3c          	shr    $0x3c,%rax
    bb57:	48 03 c3             	add    %rbx,%rax
    bb5a:	48 83 e0 f0          	and    $0xfffffffffffffff0,%rax
    bb5e:	48 89 d9             	mov    %rbx,%rcx
    bb61:	48 2b c8             	sub    %rax,%rcx
    bb64:	48 3b d8             	cmp    %rax,%rbx
    bb67:	0f 95 c2             	setne  %dl
    bb6a:	48 89 ce             	mov    %rcx,%rsi
    bb6d:	48 83 f6 10          	xor    $0x10,%rsi
    bb71:	48 33 c0             	xor    %rax,%rax
    bb74:	48 c1 ee 3f          	shr    $0x3f,%rsi
    bb78:	40 84 d6             	test   %dl,%sil
    bb7b:	48 0f 45 05 5d 01 00 	cmovne 0x15d(%rip),%rax        # bce0 <py1_access+0x1d0>
    bb82:	00 
    bb83:	4c 8d 2c 01          	lea    (%rcx,%rax,1),%r13
    bb87:	4c 8d 05 e2 3e 00 00 	lea    0x3ee2(%rip),%r8        # fa70 <pt_seq_get>
    bb8e:	4c 89 e2             	mov    %r12,%rdx
    bb91:	4c 89 ee             	mov    %r13,%rsi
    bb94:	4c 89 f7             	mov    %r14,%rdi
    bb97:	41 ff d0             	call   *%r8
    bb9a:	48 0f b6 54 24 10    	movzbq 0x10(%rsp),%rdx
    bba0:	40 80 fa 01          	rex cmp $0x1,%dl
    bba4:	0f 84 f5 00 00 00    	je     bc9f <py1_access+0x18f>
    bbaa:	4c 89 f9             	mov    %r15,%rcx
    bbad:	48 03 c8             	add    %rax,%rcx
    bbb0:	49 89 cf             	mov    %rcx,%r15
    bbb3:	40 0f 90 c6          	seto   %sil
    bbb7:	40 84 f6             	test   %sil,%sil
    bbba:	0f 85 c8 00 00 00    	jne    bc88 <py1_access+0x178>
    bbc0:	ba 07 00 00 00       	mov    $0x7,%edx
    bbc5:	4c 8d 05 84 3b 00 00 	lea    0x3b84(%rip),%r8        # f750 <pt_list_set>
    bbcc:	4c 89 ee             	mov    %r13,%rsi
    bbcf:	4c 89 f7             	mov    %r14,%rdi
    bbd2:	41 ff d0             	call   *%r8
    bbd5:	3c 01                	cmp    $0x1,%al
    bbd7:	0f 84 6a 00 00 00    	je     bc47 <py1_access+0x137>
    bbdd:	49 89 db             	mov    %rbx,%r11
    bbe0:	49 83 c3 01          	add    $0x1,%r11
    bbe4:	0f 90 c0             	seto   %al
    bbe7:	84 c0                	test   %al,%al
    bbe9:	0f 85 14 00 00 00    	jne    bc03 <py1_access+0xf3>
    bbef:	48 83 c3 01          	add    $0x1,%rbx
    bbf3:	0f 90 c0             	seto   %al
    bbf6:	84 c0                	test   %al,%al
    bbf8:	0f 85 32 00 00 00    	jne    bc30 <py1_access+0x120>
    bbfe:	e9 3c ff ff ff       	jmp    bb3f <py1_access+0x2f>
    bc03:	4c 89 f8             	mov    %r15,%rax
    bc06:	e9 03 00 00 00       	jmp    bc0e <py1_access+0xfe>
    bc0b:	48 33 c0             	xor    %rax,%rax
    bc0e:	48 8b 5c 24 20       	mov    0x20(%rsp),%rbx
    bc13:	4c 8b 64 24 28       	mov    0x28(%rsp),%r12
    bc18:	4c 8b 6c 24 30       	mov    0x30(%rsp),%r13
    bc1d:	4c 8b 74 24 38       	mov    0x38(%rsp),%r14
    bc22:	4c 8b 7c 24 40       	mov    0x40(%rsp),%r15
    bc27:	48 83 c4 50          	add    $0x50,%rsp
    bc2b:	48 89 ec             	mov    %rbp,%rsp
    bc2e:	5d                   	pop    %rbp
    bc2f:	c3                   	ret
    bc30:	48 8d 3d 25 36 04 00 	lea    0x43625(%rip),%rdi        # 4f25c <.Ldata9>
    bc37:	be 9f 00 00 00       	mov    $0x9f,%esi
    bc3c:	48 8d 05 5d 68 00 00 	lea    0x685d(%rip),%rax        # 124a0 <pt_fail>
    bc43:	ff d0                	call   *%rax
    bc45:	0f 0b                	ud2
    bc47:	b8 01 00 00 00       	mov    $0x1,%eax
    bc4c:	48 8d 35 f4 37 04 00 	lea    0x437f4(%rip),%rsi        # 4f447 <.Ldatac>
    bc53:	ba 22 00 00 00       	mov    $0x22,%edx
    bc58:	48 8d 0d 0a 38 04 00 	lea    0x4380a(%rip),%rcx        # 4f469 <.Ldatad>
    bc5f:	41 b8 41 00 00 00    	mov    $0x41,%r8d
    bc65:	4c 8d 0d 3e 38 04 00 	lea    0x4383e(%rip),%r9        # 4f4aa <.Ldatae>
    bc6c:	41 ba 25 00 00 00    	mov    $0x25,%r10d
    bc72:	48 0f b6 f8          	movzbq %al,%rdi
    bc76:	4c 89 14 24          	mov    %r10,(%rsp)
    bc7a:	48 8d 05 0f 71 00 00 	lea    0x710f(%rip),%rax        # 12d90 <pt_raise_msg>
    bc81:	ff d0                	call   *%rax
    bc83:	e9 83 ff ff ff       	jmp    bc0b <py1_access+0xfb>
    bc88:	48 8d 3d 6c 36 04 00 	lea    0x4366c(%rip),%rdi        # 4f2fb <.Ldataa>
    bc8f:	be 8b 00 00 00       	mov    $0x8b,%esi
    bc94:	48 8d 05 05 68 00 00 	lea    0x6805(%rip),%rax        # 124a0 <pt_fail>
    bc9b:	ff d0                	call   *%rax
    bc9d:	0f 0b                	ud2
    bc9f:	b8 01 00 00 00       	mov    $0x1,%eax
    bca4:	48 8d 35 2d 35 04 00 	lea    0x4352d(%rip),%rsi        # 4f1d8 <.Ldata6>
    bcab:	ba 17 00 00 00       	mov    $0x17,%edx
    bcb0:	48 8d 0d 38 35 04 00 	lea    0x43538(%rip),%rcx        # 4f1ef <.Ldata7>
    bcb7:	41 b8 42 00 00 00    	mov    $0x42,%r8d
    bcbd:	4c 8d 0d 6d 35 04 00 	lea    0x4356d(%rip),%r9        # 4f231 <.Ldata8>
    bcc4:	41 ba 2b 00 00 00    	mov    $0x2b,%r10d
    bcca:	48 0f b6 f8          	movzbq %al,%rdi
    bcce:	4c 89 14 24          	mov    %r10,(%rsp)
    bcd2:	48 8d 05 b7 70 00 00 	lea    0x70b7(%rip),%rax        # 12d90 <pt_raise_msg>
    bcd9:	ff d0                	call   *%rax
    bcdb:	e9 2b ff ff ff       	jmp    bc0b <py1_access+0xfb>
    bce0:	10 00                	adc    %al,(%rax)
    bce2:	00 00                	add    %al,(%rax)
    bce4:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:
