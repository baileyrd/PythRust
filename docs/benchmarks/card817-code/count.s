
/workspace/card817/probe-scoped/counts:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .text:

0000000000407878 <pythrust_main>:
  407878:	55                   	push   %rbp
  407879:	48 89 e5             	mov    %rsp,%rbp
  40787c:	48 81 ec 80 00 00 00 	sub    $0x80,%rsp
  407883:	48 89 5c 24 50       	mov    %rbx,0x50(%rsp)
  407888:	4c 89 64 24 58       	mov    %r12,0x58(%rsp)
  40788d:	4c 89 6c 24 60       	mov    %r13,0x60(%rsp)
  407892:	4c 89 74 24 68       	mov    %r14,0x68(%rsp)
  407897:	4c 89 7c 24 70       	mov    %r15,0x70(%rsp)
  40789c:	48 8d 3d 5d 47 04 00 	lea    0x4475d(%rip),%rdi        # 44c000 <.Ldata1>
  4078a3:	48 33 db             	xor    %rbx,%rbx
  4078a6:	48 8d 05 d3 36 04 00 	lea    0x436d3(%rip),%rax        # 44af80 <__wrap_pt_str_new>
  4078ad:	48 89 de             	mov    %rbx,%rsi
  4078b0:	ff d0                	call   *%rax
  4078b2:	4c 8d 25 47 47 04 00 	lea    0x44747(%rip),%r12        # 44c000 <.Ldata1>
  4078b9:	4c 8d 2d 41 47 04 00 	lea    0x44741(%rip),%r13        # 44c001 <.Ldata2>
  4078c0:	49 89 c7             	mov    %rax,%r15
  4078c3:	48 89 5c 24 10       	mov    %rbx,0x10(%rsp)
  4078c8:	48 89 5c 24 18       	mov    %rbx,0x18(%rsp)
  4078cd:	48 81 fb 40 42 0f 00 	cmp    $0xf4240,%rbx
  4078d4:	0f 8d 40 02 00 00    	jge    407b1a <pythrust_main+0x2a2>
  4078da:	41 be 01 00 00 00    	mov    $0x1,%r14d
  4078e0:	48 8d 05 99 36 04 00 	lea    0x43699(%rip),%rax        # 44af80 <__wrap_pt_str_new>
  4078e7:	4c 89 f6             	mov    %r14,%rsi
  4078ea:	4c 89 e7             	mov    %r12,%rdi
  4078ed:	ff d0                	call   *%rax
  4078ef:	48 89 44 24 48       	mov    %rax,0x48(%rsp)
  4078f4:	48 8d 05 d5 36 04 00 	lea    0x436d5(%rip),%rax        # 44afd0 <__wrap_pt_str_from_int>
  4078fb:	48 89 df             	mov    %rbx,%rdi
  4078fe:	ff d0                	call   *%rax
  407900:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
  407905:	48 8d 05 e4 37 04 00 	lea    0x437e4(%rip),%rax        # 44b0f0 <__wrap_pt_str_concat>
  40790c:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
  407911:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
  407916:	ff d0                	call   *%rax
  407918:	48 89 44 24 38       	mov    %rax,0x38(%rsp)
  40791d:	48 8d 05 5c 36 04 00 	lea    0x4365c(%rip),%rax        # 44af80 <__wrap_pt_str_new>
  407924:	4c 89 f6             	mov    %r14,%rsi
  407927:	4c 89 ef             	mov    %r13,%rdi
  40792a:	ff d0                	call   *%rax
  40792c:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
  407931:	48 8d 0d b8 37 04 00 	lea    0x437b8(%rip),%rcx        # 44b0f0 <__wrap_pt_str_concat>
  407938:	48 8b 74 24 30       	mov    0x30(%rsp),%rsi
  40793d:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
  407942:	ff d1                	call   *%rcx
  407944:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
  407949:	48 89 d8             	mov    %rbx,%rax
  40794c:	48 f7 2d f5 02 00 00 	imulq  0x2f5(%rip)        # 407c48 <pythrust_main+0x3d0>
  407953:	48 8d 04 1a          	lea    (%rdx,%rbx,1),%rax
  407957:	48 c1 f8 06          	sar    $0x6,%rax
  40795b:	48 89 c1             	mov    %rax,%rcx
  40795e:	48 c1 e9 3f          	shr    $0x3f,%rcx
  407962:	48 03 c1             	add    %rcx,%rax
  407965:	48 6b c8 61          	imul   $0x61,%rax,%rcx
  407969:	48 89 d8             	mov    %rbx,%rax
  40796c:	48 2b c1             	sub    %rcx,%rax
  40796f:	48 3b d9             	cmp    %rcx,%rbx
  407972:	0f 95 c1             	setne  %cl
  407975:	48 89 c6             	mov    %rax,%rsi
  407978:	48 83 f6 61          	xor    $0x61,%rsi
  40797c:	48 33 d2             	xor    %rdx,%rdx
  40797f:	48 c1 ee 3f          	shr    $0x3f,%rsi
  407983:	40 84 ce             	test   %cl,%sil
  407986:	48 0f 45 15 c2 02 00 	cmovne 0x2c2(%rip),%rdx        # 407c50 <pythrust_main+0x3d8>
  40798d:	00 
  40798e:	48 8d 3c 10          	lea    (%rax,%rdx,1),%rdi
  407992:	48 8d 05 37 36 04 00 	lea    0x43637(%rip),%rax        # 44afd0 <__wrap_pt_str_from_int>
  407999:	ff d0                	call   *%rax
  40799b:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
  4079a0:	48 8d 0d 49 37 04 00 	lea    0x43749(%rip),%rcx        # 44b0f0 <__wrap_pt_str_concat>
  4079a7:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
  4079ac:	48 8b 7c 24 20       	mov    0x20(%rsp),%rdi
  4079b1:	ff d1                	call   *%rcx
  4079b3:	49 89 c6             	mov    %rax,%r14
  4079b6:	48 8d 05 33 3f 00 00 	lea    0x3f33(%rip),%rax        # 40b8f0 <pt_str_dec>
  4079bd:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
  4079c2:	ff d0                	call   *%rax
  4079c4:	48 8d 05 25 3f 00 00 	lea    0x3f25(%rip),%rax        # 40b8f0 <pt_str_dec>
  4079cb:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
  4079d0:	ff d0                	call   *%rax
  4079d2:	48 8d 05 17 3f 00 00 	lea    0x3f17(%rip),%rax        # 40b8f0 <pt_str_dec>
  4079d9:	48 8b 7c 24 40       	mov    0x40(%rsp),%rdi
  4079de:	ff d0                	call   *%rax
  4079e0:	48 8d 05 09 3f 00 00 	lea    0x3f09(%rip),%rax        # 40b8f0 <pt_str_dec>
  4079e7:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
  4079ec:	ff d0                	call   *%rax
  4079ee:	48 8d 05 fb 3e 00 00 	lea    0x3efb(%rip),%rax        # 40b8f0 <pt_str_dec>
  4079f5:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
  4079fa:	ff d0                	call   *%rax
  4079fc:	48 8d 0d ed 3e 00 00 	lea    0x3eed(%rip),%rcx        # 40b8f0 <pt_str_dec>
  407a03:	48 8b 7c 24 20       	mov    0x20(%rsp),%rdi
  407a08:	ff d1                	call   *%rcx
  407a0a:	48 8d 15 df 3e 00 00 	lea    0x3edf(%rip),%rdx        # 40b8f0 <pt_str_dec>
  407a11:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
  407a16:	ff d2                	call   *%rdx
  407a18:	48 8d 35 31 48 00 00 	lea    0x4831(%rip),%rsi        # 40c250 <pt_str_len>
  407a1f:	4c 89 f7             	mov    %r14,%rdi
  407a22:	ff d6                	call   *%rsi
  407a24:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
  407a29:	48 03 f8             	add    %rax,%rdi
  407a2c:	48 89 7c 24 10       	mov    %rdi,0x10(%rsp)
  407a31:	40 0f 90 c6          	seto   %sil
  407a35:	40 84 f6             	test   %sil,%sil
  407a38:	0f 85 ef 01 00 00    	jne    407c2d <pythrust_main+0x3b5>
  407a3e:	48 89 d8             	mov    %rbx,%rax
  407a41:	48 f7 2d 10 02 00 00 	imulq  0x210(%rip)        # 407c58 <pythrust_main+0x3e0>
  407a48:	48 8d 34 1a          	lea    (%rdx,%rbx,1),%rsi
  407a4c:	48 c1 fe 05          	sar    $0x5,%rsi
  407a50:	48 89 f7             	mov    %rsi,%rdi
  407a53:	48 c1 ef 3f          	shr    $0x3f,%rdi
  407a57:	48 03 f7             	add    %rdi,%rsi
  407a5a:	48 6b fe 32          	imul   $0x32,%rsi,%rdi
  407a5e:	48 89 de             	mov    %rbx,%rsi
  407a61:	48 2b f7             	sub    %rdi,%rsi
  407a64:	48 3b df             	cmp    %rdi,%rbx
  407a67:	40 0f 95 c7          	setne  %dil
  407a6b:	49 89 f1             	mov    %rsi,%r9
  407a6e:	49 83 f1 32          	xor    $0x32,%r9
  407a72:	4d 33 c0             	xor    %r8,%r8
  407a75:	49 c1 e9 3f          	shr    $0x3f,%r9
  407a79:	41 84 f9             	test   %dil,%r9b
  407a7c:	4c 0f 45 05 dc 01 00 	cmovne 0x1dc(%rip),%r8        # 407c60 <pythrust_main+0x3e8>
  407a83:	00 
  407a84:	4c 03 c6             	add    %rsi,%r8
  407a87:	4d 85 c0             	test   %r8,%r8
  407a8a:	0f 85 55 00 00 00    	jne    407ae5 <pythrust_main+0x26d>
  407a90:	48 33 f6             	xor    %rsi,%rsi
  407a93:	48 8d 05 c6 35 04 00 	lea    0x435c6(%rip),%rax        # 44b060 <__wrap_pt_str_index>
  407a9a:	4c 89 f7             	mov    %r14,%rdi
  407a9d:	ff d0                	call   *%rax
  407a9f:	48 89 c2             	mov    %rax,%rdx
  407aa2:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
  407aa7:	48 85 d0             	test   %rdx,%rax
  407aaa:	0f 84 12 01 00 00    	je     407bc2 <pythrust_main+0x34a>
  407ab0:	48 8d 0d 39 36 04 00 	lea    0x43639(%rip),%rcx        # 44b0f0 <__wrap_pt_str_concat>
  407ab7:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
  407abc:	4c 89 ff             	mov    %r15,%rdi
  407abf:	ff d1                	call   *%rcx
  407ac1:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
  407ac6:	48 8d 05 23 3e 00 00 	lea    0x3e23(%rip),%rax        # 40b8f0 <pt_str_dec>
  407acd:	4c 89 ff             	mov    %r15,%rdi
  407ad0:	ff d0                	call   *%rax
  407ad2:	48 8d 05 17 3e 00 00 	lea    0x3e17(%rip),%rax        # 40b8f0 <pt_str_dec>
  407ad9:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
  407ade:	ff d0                	call   *%rax
  407ae0:	4c 8b 7c 24 20       	mov    0x20(%rsp),%r15
  407ae5:	48 89 d8             	mov    %rbx,%rax
  407ae8:	48 83 c0 01          	add    $0x1,%rax
  407aec:	0f 90 c0             	seto   %al
  407aef:	84 c0                	test   %al,%al
  407af1:	0f 84 0a 00 00 00    	je     407b01 <pythrust_main+0x289>
  407af7:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
  407afc:	e9 19 00 00 00       	jmp    407b1a <pythrust_main+0x2a2>
  407b01:	48 83 c3 01          	add    $0x1,%rbx
  407b05:	0f 90 c0             	seto   %al
  407b08:	84 c0                	test   %al,%al
  407b0a:	0f 85 9b 00 00 00    	jne    407bab <pythrust_main+0x333>
  407b10:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
  407b15:	e9 b3 fd ff ff       	jmp    4078cd <pythrust_main+0x55>
  407b1a:	48 8d 05 2f 47 00 00 	lea    0x472f(%rip),%rax        # 40c250 <pt_str_len>
  407b21:	4c 89 ff             	mov    %r15,%rdi
  407b24:	ff d0                	call   *%rax
  407b26:	48 89 c3             	mov    %rax,%rbx
  407b29:	48 8d 05 c0 6d 00 00 	lea    0x6dc0(%rip),%rax        # 40e8f0 <pt_write_int>
  407b30:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
  407b35:	ff d0                	call   *%rax
  407b37:	b8 20 00 00 00       	mov    $0x20,%eax
  407b3c:	48 0f b6 f8          	movzbq %al,%rdi
  407b40:	48 8d 05 79 6d 00 00 	lea    0x6d79(%rip),%rax        # 40e8c0 <pt_write_byte>
  407b47:	ff d0                	call   *%rax
  407b49:	48 8d 05 a0 6d 00 00 	lea    0x6da0(%rip),%rax        # 40e8f0 <pt_write_int>
  407b50:	48 89 df             	mov    %rbx,%rdi
  407b53:	ff d0                	call   *%rax
  407b55:	b8 0a 00 00 00       	mov    $0xa,%eax
  407b5a:	48 0f b6 f8          	movzbq %al,%rdi
  407b5e:	48 8d 05 5b 6d 00 00 	lea    0x6d5b(%rip),%rax        # 40e8c0 <pt_write_byte>
  407b65:	ff d0                	call   *%rax
  407b67:	48 8d 05 82 3d 00 00 	lea    0x3d82(%rip),%rax        # 40b8f0 <pt_str_dec>
  407b6e:	4c 89 ff             	mov    %r15,%rdi
  407b71:	ff d0                	call   *%rax
  407b73:	48 8d 05 76 3d 00 00 	lea    0x3d76(%rip),%rax        # 40b8f0 <pt_str_dec>
  407b7a:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
  407b7f:	ff d0                	call   *%rax
  407b81:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
  407b86:	4c 8b 64 24 58       	mov    0x58(%rsp),%r12
  407b8b:	4c 8b 6c 24 60       	mov    0x60(%rsp),%r13
  407b90:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
  407b95:	4c 8b 7c 24 70       	mov    0x70(%rsp),%r15
  407b9a:	48 81 c4 80 00 00 00 	add    $0x80,%rsp
  407ba1:	48 89 ec             	mov    %rbp,%rsp
  407ba4:	5d                   	pop    %rbp
  407ba5:	c3                   	ret
  407ba6:	e9 53 00 00 00       	jmp    407bfe <pythrust_main+0x386>
  407bab:	48 8d 3d 04 46 04 00 	lea    0x44604(%rip),%rdi        # 44c1b6 <.Ldata9>
  407bb2:	be 96 00 00 00       	mov    $0x96,%esi
  407bb7:	48 8d 05 a2 6c 00 00 	lea    0x6ca2(%rip),%rax        # 40e860 <pt_fail>
  407bbe:	ff d0                	call   *%rax
  407bc0:	0f 0b                	ud2
  407bc2:	b8 01 00 00 00       	mov    $0x1,%eax
  407bc7:	48 8d 35 13 47 04 00 	lea    0x44713(%rip),%rsi        # 44c2e1 <.Ldatab>
  407bce:	ba 19 00 00 00       	mov    $0x19,%edx
  407bd3:	48 8d 0d 20 47 04 00 	lea    0x44720(%rip),%rcx        # 44c2fa <.Ldatac>
  407bda:	41 b8 41 00 00 00    	mov    $0x41,%r8d
  407be0:	4c 8d 0d 54 47 04 00 	lea    0x44754(%rip),%r9        # 44c33b <.Ldatad>
  407be7:	41 ba 33 00 00 00    	mov    $0x33,%r10d
  407bed:	48 0f b6 f8          	movzbq %al,%rdi
  407bf1:	4c 89 14 24          	mov    %r10,(%rsp)
  407bf5:	48 8d 05 54 75 00 00 	lea    0x7554(%rip),%rax        # 40f150 <pt_raise_msg>
  407bfc:	ff d0                	call   *%rax
  407bfe:	48 8d 05 eb 3c 00 00 	lea    0x3ceb(%rip),%rax        # 40b8f0 <pt_str_dec>
  407c05:	4c 89 ff             	mov    %r15,%rdi
  407c08:	ff d0                	call   *%rax
  407c0a:	48 8d 05 df 3c 00 00 	lea    0x3cdf(%rip),%rax        # 40b8f0 <pt_str_dec>
  407c11:	4c 89 f7             	mov    %r14,%rdi
  407c14:	ff d0                	call   *%rax
  407c16:	48 33 ff             	xor    %rdi,%rdi
  407c19:	48 8d 05 d0 3c 00 00 	lea    0x3cd0(%rip),%rax        # 40b8f0 <pt_str_dec>
  407c20:	ff d0                	call   *%rax
  407c22:	48 8d 05 17 76 00 00 	lea    0x7617(%rip),%rax        # 40f240 <pt_uncaught>
  407c29:	ff d0                	call   *%rax
  407c2b:	0f 0b                	ud2
  407c2d:	48 8d 3d 8b 44 04 00 	lea    0x4448b(%rip),%rdi        # 44c0bf <.Ldata6>
  407c34:	be 90 00 00 00       	mov    $0x90,%esi
  407c39:	48 8d 05 20 6c 00 00 	lea    0x6c20(%rip),%rax        # 40e860 <pt_fail>
  407c40:	ff d0                	call   *%rax
  407c42:	0f 0b                	ud2
  407c44:	00 00                	add    %al,(%rax)
  407c46:	00 00                	add    %al,(%rax)
  407c48:	e9 a8 c0 17 57       	jmp    57583cf5 <_end+0x57121dad>
  407c4d:	3f                   	(bad)
  407c4e:	e8 a8 61 00 00       	call   40ddfb <_RINvNtCs40k4W9msRzi_5alloc3str17join_generic_copyehReECsk1kE6r7WBHz_11pythrust_rt+0x46b>
  407c53:	00 00                	add    %al,(%rax)
  407c55:	00 00                	add    %al,(%rax)
  407c57:	00 0b                	add    %cl,(%rbx)
  407c59:	d7                   	xlat   %ds:(%rbx)
  407c5a:	a3 70 3d 0a d7 a3 32 	movabs %eax,0x32a3d70a3d70
  407c61:	00 00 
  407c63:	00 00                	add    %al,(%rax)
  407c65:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:

/workspace/card817/probe-scoped/counts:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .text:

000000000040b7b0 <pt_str_concat>:
  40b7b0:	55                   	push   %rbp
  40b7b1:	41 57                	push   %r15
  40b7b3:	41 56                	push   %r14
  40b7b5:	41 55                	push   %r13
  40b7b7:	41 54                	push   %r12
  40b7b9:	53                   	push   %rbx
  40b7ba:	48 83 ec 38          	sub    $0x38,%rsp
  40b7be:	48 89 f3             	mov    %rsi,%rbx
  40b7c1:	49 89 fe             	mov    %rdi,%r14
  40b7c4:	0f 10 07             	movups (%rdi),%xmm0
  40b7c7:	0f 10 0e             	movups (%rsi),%xmm1
  40b7ca:	0f 11 44 24 18       	movups %xmm0,0x18(%rsp)
  40b7cf:	0f 11 4c 24 28       	movups %xmm1,0x28(%rsp)
  40b7d4:	48 89 e7             	mov    %rsp,%rdi
  40b7d7:	48 8d 74 24 18       	lea    0x18(%rsp),%rsi
  40b7dc:	ba 02 00 00 00       	mov    $0x2,%edx
  40b7e1:	b9 01 00 00 00       	mov    $0x1,%ecx
  40b7e6:	45 31 c0             	xor    %r8d,%r8d
  40b7e9:	e8 a2 21 00 00       	call   40d990 <_RINvNtCs40k4W9msRzi_5alloc3str17join_generic_copyehReECsk1kE6r7WBHz_11pythrust_rt>
  40b7ee:	49 8b 6e 18          	mov    0x18(%r14),%rbp
  40b7f2:	4c 8b 6b 18          	mov    0x18(%rbx),%r13
  40b7f6:	4c 8b 3c 24          	mov    (%rsp),%r15
  40b7fa:	48 8b 5c 24 10       	mov    0x10(%rsp),%rbx
  40b7ff:	49 39 df             	cmp    %rbx,%r15
  40b802:	76 36                	jbe    40b83a <pt_str_concat+0x8a>
  40b804:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
  40b809:	48 85 db             	test   %rbx,%rbx
  40b80c:	74 33                	je     40b841 <pt_str_concat+0x91>
  40b80e:	ba 01 00 00 00       	mov    $0x1,%edx
  40b813:	4c 89 e7             	mov    %r12,%rdi
  40b816:	4c 89 fe             	mov    %r15,%rsi
  40b819:	48 89 d9             	mov    %rbx,%rcx
  40b81c:	ff 15 de 54 05 00    	call   *0x554de(%rip)        # 460d00 <_DYNAMIC+0x780>
  40b822:	49 89 c6             	mov    %rax,%r14
  40b825:	48 85 c0             	test   %rax,%rax
  40b828:	75 2e                	jne    40b858 <pt_str_concat+0xa8>
  40b82a:	bf 01 00 00 00       	mov    $0x1,%edi
  40b82f:	48 89 de             	mov    %rbx,%rsi
  40b832:	ff 15 b0 53 05 00    	call   *0x553b0(%rip)        # 460be8 <_DYNAMIC+0x668>
  40b838:	eb 6e                	jmp    40b8a8 <pt_str_concat+0xf8>
  40b83a:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
  40b83f:	eb 17                	jmp    40b858 <pt_str_concat+0xa8>
  40b841:	41 be 01 00 00 00    	mov    $0x1,%r14d
  40b847:	ba 01 00 00 00       	mov    $0x1,%edx
  40b84c:	4c 89 e7             	mov    %r12,%rdi
  40b84f:	4c 89 fe             	mov    %r15,%rsi
  40b852:	ff 15 d0 56 05 00    	call   *0x556d0(%rip)        # 460f28 <_DYNAMIC+0x9a8>
  40b858:	ff 15 c2 51 05 00    	call   *0x551c2(%rip)        # 460a20 <_DYNAMIC+0x4a0>
  40b85e:	bf 20 00 00 00       	mov    $0x20,%edi
  40b863:	be 08 00 00 00       	mov    $0x8,%esi
  40b868:	ff 15 f2 51 05 00    	call   *0x551f2(%rip)        # 460a60 <_DYNAMIC+0x4e0>
  40b86e:	48 85 c0             	test   %rax,%rax
  40b871:	74 25                	je     40b898 <pt_str_concat+0xe8>
  40b873:	49 01 ed             	add    %rbp,%r13
  40b876:	4c 89 30             	mov    %r14,(%rax)
  40b879:	48 89 58 08          	mov    %rbx,0x8(%rax)
  40b87d:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
  40b884:	00 
  40b885:	4c 89 68 18          	mov    %r13,0x18(%rax)
  40b889:	48 83 c4 38          	add    $0x38,%rsp
  40b88d:	5b                   	pop    %rbx
  40b88e:	41 5c                	pop    %r12
  40b890:	41 5d                	pop    %r13
  40b892:	41 5e                	pop    %r14
  40b894:	41 5f                	pop    %r15
  40b896:	5d                   	pop    %rbp
  40b897:	c3                   	ret
  40b898:	bf 08 00 00 00       	mov    $0x8,%edi
  40b89d:	be 20 00 00 00       	mov    $0x20,%esi
  40b8a2:	ff 15 b0 53 05 00    	call   *0x553b0(%rip)        # 460c58 <_DYNAMIC+0x6d8>
  40b8a8:	0f 0b                	ud2
  40b8aa:	ba 01 00 00 00       	mov    $0x1,%edx
  40b8af:	4c 89 e7             	mov    %r12,%rdi
  40b8b2:	4c 89 fe             	mov    %r15,%rsi
  40b8b5:	ff 15 6d 56 05 00    	call   *0x5566d(%rip)        # 460f28 <_DYNAMIC+0x9a8>
  40b8bb:	ff 15 e7 4f 05 00    	call   *0x54fe7(%rip)        # 4608a8 <_DYNAMIC+0x328>
  40b8c1:	ff 15 e1 4f 05 00    	call   *0x54fe1(%rip)        # 4608a8 <_DYNAMIC+0x328>
  40b8c7:	48 85 db             	test   %rbx,%rbx
  40b8ca:	75 06                	jne    40b8d2 <pt_str_concat+0x122>
  40b8cc:	ff 15 d6 4f 05 00    	call   *0x54fd6(%rip)        # 4608a8 <_DYNAMIC+0x328>
  40b8d2:	ba 01 00 00 00       	mov    $0x1,%edx
  40b8d7:	4c 89 f7             	mov    %r14,%rdi
  40b8da:	48 89 de             	mov    %rbx,%rsi
  40b8dd:	ff 15 45 56 05 00    	call   *0x55645(%rip)        # 460f28 <_DYNAMIC+0x9a8>
  40b8e3:	ff 15 bf 4f 05 00    	call   *0x54fbf(%rip)        # 4608a8 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card817/probe-scoped/counts:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .text:

000000000040b970 <pt_str_from_int>:
  40b970:	41 57                	push   %r15
  40b972:	41 56                	push   %r14
  40b974:	41 54                	push   %r12
  40b976:	53                   	push   %rbx
  40b977:	48 83 ec 38          	sub    $0x38,%rsp
  40b97b:	0f 28 05 5e 3f 04 00 	movaps 0x43f5e(%rip),%xmm0        # 44f8e0 <anon.13df560d6b83da2c10641ad9b2156d4a.31.llvm.366253514071984098+0x51>
  40b982:	0f 29 04 24          	movaps %xmm0,(%rsp)
  40b986:	c7 44 24 0f 30 30 30 	movl   $0x30303030,0xf(%rsp)
  40b98d:	30 
  40b98e:	48 89 f9             	mov    %rdi,%rcx
  40b991:	48 f7 d9             	neg    %rcx
  40b994:	48 0f 48 cf          	cmovs  %rdi,%rcx
  40b998:	48 ba cd cc cc cc cc 	movabs $0xcccccccccccccccd,%rdx
  40b99f:	cc cc cc 
  40b9a2:	48 89 c8             	mov    %rcx,%rax
  40b9a5:	48 f7 e2             	mul    %rdx
  40b9a8:	48 c1 ea 03          	shr    $0x3,%rdx
  40b9ac:	8d 04 12             	lea    (%rdx,%rdx,1),%eax
  40b9af:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40b9b2:	89 ce                	mov    %ecx,%esi
  40b9b4:	29 c6                	sub    %eax,%esi
  40b9b6:	40 80 ce 30          	or     $0x30,%sil
  40b9ba:	40 88 74 24 13       	mov    %sil,0x13(%rsp)
  40b9bf:	48 83 f9 0a          	cmp    $0xa,%rcx
  40b9c3:	73 0f                	jae    40b9d4 <pt_str_from_int+0x64>
  40b9c5:	b8 13 00 00 00       	mov    $0x13,%eax
  40b9ca:	b9 12 00 00 00       	mov    $0x12,%ecx
  40b9cf:	e9 f1 04 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40b9d4:	49 b8 9a 99 99 99 99 	movabs $0x199999999999999a,%r8
  40b9db:	99 99 19 
  40b9de:	48 89 d0             	mov    %rdx,%rax
  40b9e1:	48 89 d6             	mov    %rdx,%rsi
  40b9e4:	49 f7 e0             	mul    %r8
  40b9e7:	01 d2                	add    %edx,%edx
  40b9e9:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40b9ec:	29 c6                	sub    %eax,%esi
  40b9ee:	40 80 ce 30          	or     $0x30,%sil
  40b9f2:	40 88 74 24 12       	mov    %sil,0x12(%rsp)
  40b9f7:	48 83 f9 64          	cmp    $0x64,%rcx
  40b9fb:	73 0f                	jae    40ba0c <pt_str_from_int+0x9c>
  40b9fd:	b8 12 00 00 00       	mov    $0x12,%eax
  40ba02:	b9 11 00 00 00       	mov    $0x11,%ecx
  40ba07:	e9 b9 04 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40ba0c:	48 89 c8             	mov    %rcx,%rax
  40ba0f:	48 c1 e8 02          	shr    $0x2,%rax
  40ba13:	48 ba c3 f5 28 5c 8f 	movabs $0x28f5c28f5c28f5c3,%rdx
  40ba1a:	c2 f5 28 
  40ba1d:	48 f7 e2             	mul    %rdx
  40ba20:	48 89 d6             	mov    %rdx,%rsi
  40ba23:	48 c1 ee 02          	shr    $0x2,%rsi
  40ba27:	48 89 f0             	mov    %rsi,%rax
  40ba2a:	49 f7 e0             	mul    %r8
  40ba2d:	01 d2                	add    %edx,%edx
  40ba2f:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40ba32:	29 c6                	sub    %eax,%esi
  40ba34:	40 80 ce 30          	or     $0x30,%sil
  40ba38:	40 88 74 24 11       	mov    %sil,0x11(%rsp)
  40ba3d:	48 81 f9 e8 03 00 00 	cmp    $0x3e8,%rcx
  40ba44:	73 0f                	jae    40ba55 <pt_str_from_int+0xe5>
  40ba46:	b8 11 00 00 00       	mov    $0x11,%eax
  40ba4b:	b9 10 00 00 00       	mov    $0x10,%ecx
  40ba50:	e9 70 04 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40ba55:	48 89 c8             	mov    %rcx,%rax
  40ba58:	48 c1 e8 03          	shr    $0x3,%rax
  40ba5c:	48 ba cf f7 53 e3 a5 	movabs $0x20c49ba5e353f7cf,%rdx
  40ba63:	9b c4 20 
  40ba66:	48 f7 e2             	mul    %rdx
  40ba69:	48 89 d6             	mov    %rdx,%rsi
  40ba6c:	48 c1 ee 04          	shr    $0x4,%rsi
  40ba70:	48 89 f0             	mov    %rsi,%rax
  40ba73:	49 f7 e0             	mul    %r8
  40ba76:	01 d2                	add    %edx,%edx
  40ba78:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40ba7b:	29 c6                	sub    %eax,%esi
  40ba7d:	40 80 ce 30          	or     $0x30,%sil
  40ba81:	40 88 74 24 10       	mov    %sil,0x10(%rsp)
  40ba86:	48 81 f9 10 27 00 00 	cmp    $0x2710,%rcx
  40ba8d:	73 0f                	jae    40ba9e <pt_str_from_int+0x12e>
  40ba8f:	b8 10 00 00 00       	mov    $0x10,%eax
  40ba94:	b9 0f 00 00 00       	mov    $0xf,%ecx
  40ba99:	e9 27 04 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40ba9e:	48 ba 4b 59 86 38 d6 	movabs $0x346dc5d63886594b,%rdx
  40baa5:	c5 6d 34 
  40baa8:	48 89 c8             	mov    %rcx,%rax
  40baab:	48 f7 e2             	mul    %rdx
  40baae:	48 89 d6             	mov    %rdx,%rsi
  40bab1:	48 c1 ee 0b          	shr    $0xb,%rsi
  40bab5:	48 89 f0             	mov    %rsi,%rax
  40bab8:	49 f7 e0             	mul    %r8
  40babb:	01 d2                	add    %edx,%edx
  40babd:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40bac0:	29 c6                	sub    %eax,%esi
  40bac2:	40 80 ce 30          	or     $0x30,%sil
  40bac6:	40 88 74 24 0f       	mov    %sil,0xf(%rsp)
  40bacb:	48 81 f9 a0 86 01 00 	cmp    $0x186a0,%rcx
  40bad2:	73 0f                	jae    40bae3 <pt_str_from_int+0x173>
  40bad4:	b8 0f 00 00 00       	mov    $0xf,%eax
  40bad9:	b9 0e 00 00 00       	mov    $0xe,%ecx
  40bade:	e9 e2 03 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bae3:	48 89 c8             	mov    %rcx,%rax
  40bae6:	48 c1 e8 05          	shr    $0x5,%rax
  40baea:	48 ba 43 78 b4 71 c4 	movabs $0xa7c5ac471b47843,%rdx
  40baf1:	5a 7c 0a 
  40baf4:	48 f7 e2             	mul    %rdx
  40baf7:	48 89 d6             	mov    %rdx,%rsi
  40bafa:	48 c1 ee 07          	shr    $0x7,%rsi
  40bafe:	48 89 f0             	mov    %rsi,%rax
  40bb01:	49 f7 e0             	mul    %r8
  40bb04:	01 d2                	add    %edx,%edx
  40bb06:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40bb09:	29 c6                	sub    %eax,%esi
  40bb0b:	40 80 ce 30          	or     $0x30,%sil
  40bb0f:	40 88 74 24 0e       	mov    %sil,0xe(%rsp)
  40bb14:	48 81 f9 40 42 0f 00 	cmp    $0xf4240,%rcx
  40bb1b:	73 0f                	jae    40bb2c <pt_str_from_int+0x1bc>
  40bb1d:	b8 0e 00 00 00       	mov    $0xe,%eax
  40bb22:	b9 0d 00 00 00       	mov    $0xd,%ecx
  40bb27:	e9 99 03 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bb2c:	48 ba db 34 b6 d7 82 	movabs $0x431bde82d7b634db,%rdx
  40bb33:	de 1b 43 
  40bb36:	48 89 c8             	mov    %rcx,%rax
  40bb39:	48 f7 e2             	mul    %rdx
  40bb3c:	48 89 d6             	mov    %rdx,%rsi
  40bb3f:	48 c1 ee 12          	shr    $0x12,%rsi
  40bb43:	48 89 f0             	mov    %rsi,%rax
  40bb46:	49 f7 e0             	mul    %r8
  40bb49:	01 d2                	add    %edx,%edx
  40bb4b:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40bb4e:	29 c6                	sub    %eax,%esi
  40bb50:	40 80 ce 30          	or     $0x30,%sil
  40bb54:	40 88 74 24 0d       	mov    %sil,0xd(%rsp)
  40bb59:	48 81 f9 80 96 98 00 	cmp    $0x989680,%rcx
  40bb60:	73 0f                	jae    40bb71 <pt_str_from_int+0x201>
  40bb62:	b8 0d 00 00 00       	mov    $0xd,%eax
  40bb67:	b9 0c 00 00 00       	mov    $0xc,%ecx
  40bb6c:	e9 54 03 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bb71:	48 ba bd 42 7a e5 d5 	movabs $0xd6bf94d5e57a42bd,%rdx
  40bb78:	94 bf d6 
  40bb7b:	48 89 c8             	mov    %rcx,%rax
  40bb7e:	48 f7 e2             	mul    %rdx
  40bb81:	48 89 d6             	mov    %rdx,%rsi
  40bb84:	48 c1 ee 17          	shr    $0x17,%rsi
  40bb88:	48 89 f0             	mov    %rsi,%rax
  40bb8b:	49 f7 e0             	mul    %r8
  40bb8e:	01 d2                	add    %edx,%edx
  40bb90:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40bb93:	29 c6                	sub    %eax,%esi
  40bb95:	40 80 ce 30          	or     $0x30,%sil
  40bb99:	40 88 74 24 0c       	mov    %sil,0xc(%rsp)
  40bb9e:	48 81 f9 00 e1 f5 05 	cmp    $0x5f5e100,%rcx
  40bba5:	73 0f                	jae    40bbb6 <pt_str_from_int+0x246>
  40bba7:	b8 0c 00 00 00       	mov    $0xc,%eax
  40bbac:	b9 0b 00 00 00       	mov    $0xb,%ecx
  40bbb1:	e9 0f 03 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bbb6:	48 ba fd ce 61 84 11 	movabs $0xabcc77118461cefd,%rdx
  40bbbd:	77 cc ab 
  40bbc0:	48 89 c8             	mov    %rcx,%rax
  40bbc3:	48 f7 e2             	mul    %rdx
  40bbc6:	48 89 d6             	mov    %rdx,%rsi
  40bbc9:	48 c1 ee 1a          	shr    $0x1a,%rsi
  40bbcd:	48 89 f0             	mov    %rsi,%rax
  40bbd0:	49 f7 e0             	mul    %r8
  40bbd3:	01 d2                	add    %edx,%edx
  40bbd5:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40bbd8:	29 c6                	sub    %eax,%esi
  40bbda:	40 80 ce 30          	or     $0x30,%sil
  40bbde:	40 88 74 24 0b       	mov    %sil,0xb(%rsp)
  40bbe3:	48 81 f9 00 ca 9a 3b 	cmp    $0x3b9aca00,%rcx
  40bbea:	73 0f                	jae    40bbfb <pt_str_from_int+0x28b>
  40bbec:	b8 0b 00 00 00       	mov    $0xb,%eax
  40bbf1:	b9 0a 00 00 00       	mov    $0xa,%ecx
  40bbf6:	e9 ca 02 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bbfb:	48 89 c8             	mov    %rcx,%rax
  40bbfe:	48 c1 e8 09          	shr    $0x9,%rax
  40bc02:	48 ba 53 5a 9b a0 2f 	movabs $0x44b82fa09b5a53,%rdx
  40bc09:	b8 44 00 
  40bc0c:	48 f7 e2             	mul    %rdx
  40bc0f:	48 89 d6             	mov    %rdx,%rsi
  40bc12:	48 c1 ee 0b          	shr    $0xb,%rsi
  40bc16:	48 89 f0             	mov    %rsi,%rax
  40bc19:	49 f7 e0             	mul    %r8
  40bc1c:	01 d2                	add    %edx,%edx
  40bc1e:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40bc21:	29 c6                	sub    %eax,%esi
  40bc23:	40 80 ce 30          	or     $0x30,%sil
  40bc27:	40 88 74 24 0a       	mov    %sil,0xa(%rsp)
  40bc2c:	48 89 c8             	mov    %rcx,%rax
  40bc2f:	48 c1 e8 0a          	shr    $0xa,%rax
  40bc33:	48 3d f9 02 95 00    	cmp    $0x9502f9,%rax
  40bc39:	73 0f                	jae    40bc4a <pt_str_from_int+0x2da>
  40bc3b:	b8 0a 00 00 00       	mov    $0xa,%eax
  40bc40:	b9 09 00 00 00       	mov    $0x9,%ecx
  40bc45:	e9 7b 02 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bc4a:	48 ba bf d5 ed bd ce 	movabs $0xdbe6fecebdedd5bf,%rdx
  40bc51:	fe e6 db 
  40bc54:	48 89 c8             	mov    %rcx,%rax
  40bc57:	48 f7 e2             	mul    %rdx
  40bc5a:	48 c1 ea 21          	shr    $0x21,%rdx
  40bc5e:	48 69 c2 67 66 66 66 	imul   $0x66666667,%rdx,%rax
  40bc65:	48 c1 e8 22          	shr    $0x22,%rax
  40bc69:	01 c0                	add    %eax,%eax
  40bc6b:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bc6e:	29 c2                	sub    %eax,%edx
  40bc70:	80 ca 30             	or     $0x30,%dl
  40bc73:	88 54 24 09          	mov    %dl,0x9(%rsp)
  40bc77:	48 89 c8             	mov    %rcx,%rax
  40bc7a:	48 c1 e8 0b          	shr    $0xb,%rax
  40bc7e:	48 3d dd 0e e9 02    	cmp    $0x2e90edd,%rax
  40bc84:	73 0f                	jae    40bc95 <pt_str_from_int+0x325>
  40bc86:	b8 09 00 00 00       	mov    $0x9,%eax
  40bc8b:	b9 08 00 00 00       	mov    $0x8,%ecx
  40bc90:	e9 30 02 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bc95:	48 ba ff aa 24 cb 0b 	movabs $0xafebff0bcb24aaff,%rdx
  40bc9c:	ff eb af 
  40bc9f:	48 89 c8             	mov    %rcx,%rax
  40bca2:	48 f7 e2             	mul    %rdx
  40bca5:	48 c1 ea 24          	shr    $0x24,%rdx
  40bca9:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
  40bcb0:	48 c1 e8 20          	shr    $0x20,%rax
  40bcb4:	01 c0                	add    %eax,%eax
  40bcb6:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bcb9:	29 c2                	sub    %eax,%edx
  40bcbb:	80 ca 30             	or     $0x30,%dl
  40bcbe:	88 54 24 08          	mov    %dl,0x8(%rsp)
  40bcc2:	48 89 c8             	mov    %rcx,%rax
  40bcc5:	48 c1 e8 0c          	shr    $0xc,%rax
  40bcc9:	48 3d 51 4a 8d 0e    	cmp    $0xe8d4a51,%rax
  40bccf:	73 0f                	jae    40bce0 <pt_str_from_int+0x370>
  40bcd1:	b8 08 00 00 00       	mov    $0x8,%eax
  40bcd6:	b9 07 00 00 00       	mov    $0x7,%ecx
  40bcdb:	e9 e5 01 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bce0:	48 ba 33 22 d4 5b 02 	movabs $0x232f33025bd42233,%rdx
  40bce7:	33 2f 23 
  40bcea:	48 89 c8             	mov    %rcx,%rax
  40bced:	48 f7 e2             	mul    %rdx
  40bcf0:	48 c1 ea 25          	shr    $0x25,%rdx
  40bcf4:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
  40bcfb:	48 c1 e8 20          	shr    $0x20,%rax
  40bcff:	01 c0                	add    %eax,%eax
  40bd01:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bd04:	29 c2                	sub    %eax,%edx
  40bd06:	80 ca 30             	or     $0x30,%dl
  40bd09:	88 54 24 07          	mov    %dl,0x7(%rsp)
  40bd0d:	48 89 c8             	mov    %rcx,%rax
  40bd10:	48 c1 e8 0d          	shr    $0xd,%rax
  40bd14:	48 3d 95 73 c2 48    	cmp    $0x48c27395,%rax
  40bd1a:	73 0f                	jae    40bd2b <pt_str_from_int+0x3bb>
  40bd1c:	b8 07 00 00 00       	mov    $0x7,%eax
  40bd21:	b9 06 00 00 00       	mov    $0x6,%ecx
  40bd26:	e9 9a 01 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bd2b:	48 ba 85 03 ed 92 d0 	movabs $0x384b84d092ed0385,%rdx
  40bd32:	84 4b 38 
  40bd35:	48 89 c8             	mov    %rcx,%rax
  40bd38:	48 f7 e2             	mul    %rdx
  40bd3b:	48 c1 ea 29          	shr    $0x29,%rdx
  40bd3f:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
  40bd46:	48 c1 e8 20          	shr    $0x20,%rax
  40bd4a:	01 c0                	add    %eax,%eax
  40bd4c:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bd4f:	29 c2                	sub    %eax,%edx
  40bd51:	80 ca 30             	or     $0x30,%dl
  40bd54:	88 54 24 06          	mov    %dl,0x6(%rsp)
  40bd58:	48 b8 00 40 7a 10 f3 	movabs $0x5af3107a4000,%rax
  40bd5f:	5a 00 00 
  40bd62:	48 39 c1             	cmp    %rax,%rcx
  40bd65:	73 0f                	jae    40bd76 <pt_str_from_int+0x406>
  40bd67:	b8 06 00 00 00       	mov    $0x6,%eax
  40bd6c:	b9 05 00 00 00       	mov    $0x5,%ecx
  40bd71:	e9 4f 01 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bd76:	48 ba 81 cd 95 50 c3 	movabs $0xb424dc35095cd81,%rdx
  40bd7d:	4d 42 0b 
  40bd80:	48 89 c8             	mov    %rcx,%rax
  40bd83:	48 f7 e2             	mul    %rdx
  40bd86:	48 c1 ea 2a          	shr    $0x2a,%rdx
  40bd8a:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
  40bd91:	48 c1 e8 20          	shr    $0x20,%rax
  40bd95:	01 c0                	add    %eax,%eax
  40bd97:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bd9a:	29 c2                	sub    %eax,%edx
  40bd9c:	80 ca 30             	or     $0x30,%dl
  40bd9f:	88 54 24 05          	mov    %dl,0x5(%rsp)
  40bda3:	48 b8 00 80 c6 a4 7e 	movabs $0x38d7ea4c68000,%rax
  40bdaa:	8d 03 00 
  40bdad:	48 39 c1             	cmp    %rax,%rcx
  40bdb0:	73 0f                	jae    40bdc1 <pt_str_from_int+0x451>
  40bdb2:	b8 05 00 00 00       	mov    $0x5,%eax
  40bdb7:	b9 04 00 00 00       	mov    $0x4,%ecx
  40bdbc:	e9 04 01 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40bdc1:	48 89 c8             	mov    %rcx,%rax
  40bdc4:	48 c1 e8 0f          	shr    $0xf,%rax
  40bdc8:	48 ba c3 ea dc f3 75 	movabs $0x24075f3dceac3,%rdx
  40bdcf:	40 02 00 
  40bdd2:	48 f7 e2             	mul    %rdx
  40bdd5:	48 c1 ea 14          	shr    $0x14,%rdx
  40bdd9:	69 c2 67 66 00 00    	imul   $0x6667,%edx,%eax
  40bddf:	c1 e8 11             	shr    $0x11,%eax
  40bde2:	83 e0 fe             	and    $0xfffffffe,%eax
  40bde5:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bde8:	29 c2                	sub    %eax,%edx
  40bdea:	80 ca 30             	or     $0x30,%dl
  40bded:	88 54 24 04          	mov    %dl,0x4(%rsp)
  40bdf1:	48 b8 00 00 c1 6f f2 	movabs $0x2386f26fc10000,%rax
  40bdf8:	86 23 00 
  40bdfb:	48 39 c1             	cmp    %rax,%rcx
  40bdfe:	73 0f                	jae    40be0f <pt_str_from_int+0x49f>
  40be00:	b8 04 00 00 00       	mov    $0x4,%eax
  40be05:	b9 03 00 00 00       	mov    $0x3,%ecx
  40be0a:	e9 b6 00 00 00       	jmp    40bec5 <pt_str_from_int+0x555>
  40be0f:	48 ba 57 78 13 b1 2f 	movabs $0x39a5652fb1137857,%rdx
  40be16:	65 a5 39 
  40be19:	48 89 c8             	mov    %rcx,%rax
  40be1c:	48 f7 e2             	mul    %rdx
  40be1f:	48 c1 ea 33          	shr    $0x33,%rdx
  40be23:	69 c2 9a 19 00 00    	imul   $0x199a,%edx,%eax
  40be29:	c1 e8 0f             	shr    $0xf,%eax
  40be2c:	83 e0 fe             	and    $0xfffffffe,%eax
  40be2f:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40be32:	29 c2                	sub    %eax,%edx
  40be34:	80 ca 30             	or     $0x30,%dl
  40be37:	88 54 24 03          	mov    %dl,0x3(%rsp)
  40be3b:	48 b8 00 00 8a 5d 78 	movabs $0x16345785d8a0000,%rax
  40be42:	45 63 01 
  40be45:	48 39 c1             	cmp    %rax,%rcx
  40be48:	73 0c                	jae    40be56 <pt_str_from_int+0x4e6>
  40be4a:	b8 03 00 00 00       	mov    $0x3,%eax
  40be4f:	b9 02 00 00 00       	mov    $0x2,%ecx
  40be54:	eb 6f                	jmp    40bec5 <pt_str_from_int+0x555>
  40be56:	48 89 c8             	mov    %rcx,%rax
  40be59:	48 c1 e8 11          	shr    $0x11,%rax
  40be5d:	48 ba 53 1b 19 d5 3b 	movabs $0x5c3bd5191b53,%rdx
  40be64:	5c 00 00 
  40be67:	48 f7 e2             	mul    %rdx
  40be6a:	c1 ea 16             	shr    $0x16,%edx
  40be6d:	69 c2 cd 00 00 00    	imul   $0xcd,%edx,%eax
  40be73:	c1 e8 0a             	shr    $0xa,%eax
  40be76:	83 e0 fe             	and    $0xfffffffe,%eax
  40be79:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40be7c:	28 c2                	sub    %al,%dl
  40be7e:	80 ca 30             	or     $0x30,%dl
  40be81:	88 54 24 02          	mov    %dl,0x2(%rsp)
  40be85:	48 b8 00 00 64 a7 b3 	movabs $0xde0b6b3a7640000,%rax
  40be8c:	b6 e0 0d 
  40be8f:	48 39 c1             	cmp    %rax,%rcx
  40be92:	73 0c                	jae    40bea0 <pt_str_from_int+0x530>
  40be94:	b8 02 00 00 00       	mov    $0x2,%eax
  40be99:	b9 01 00 00 00       	mov    $0x1,%ecx
  40be9e:	eb 25                	jmp    40bec5 <pt_str_from_int+0x555>
  40bea0:	48 c1 e9 12          	shr    $0x12,%rcx
  40bea4:	48 ba 0f 49 47 77 c9 	movabs $0x49c97747490f,%rdx
  40beab:	49 00 00 
  40beae:	48 89 c8             	mov    %rcx,%rax
  40beb1:	48 f7 e2             	mul    %rdx
  40beb4:	c1 ea 18             	shr    $0x18,%edx
  40beb7:	80 ca 30             	or     $0x30,%dl
  40beba:	88 54 24 01          	mov    %dl,0x1(%rsp)
  40bebe:	b8 01 00 00 00       	mov    $0x1,%eax
  40bec3:	31 c9                	xor    %ecx,%ecx
  40bec5:	48 85 ff             	test   %rdi,%rdi
  40bec8:	79 07                	jns    40bed1 <pt_str_from_int+0x561>
  40beca:	c6 04 0c 2d          	movb   $0x2d,(%rsp,%rcx,1)
  40bece:	48 89 c8             	mov    %rcx,%rax
  40bed1:	ba 14 00 00 00       	mov    $0x14,%edx
  40bed6:	48 29 c2             	sub    %rax,%rdx
  40bed9:	48 8d 34 04          	lea    (%rsp,%rax,1),%rsi
  40bedd:	48 8d 7c 24 20       	lea    0x20(%rsp),%rdi
  40bee2:	ff 15 10 4b 05 00    	call   *0x54b10(%rip)        # 4609f8 <_DYNAMIC+0x478>
  40bee8:	80 7c 24 20 00       	cmpb   $0x0,0x20(%rsp)
  40beed:	0f 85 98 00 00 00    	jne    40bf8b <pt_str_from_int+0x61b>
  40bef3:	48 8b 5c 24 30       	mov    0x30(%rsp),%rbx
  40bef8:	48 85 db             	test   %rbx,%rbx
  40befb:	79 14                	jns    40bf11 <pt_str_from_int+0x5a1>
  40befd:	45 31 e4             	xor    %r12d,%r12d
  40bf00:	4c 89 e7             	mov    %r12,%rdi
  40bf03:	48 89 de             	mov    %rbx,%rsi
  40bf06:	ff 15 dc 4c 05 00    	call   *0x54cdc(%rip)        # 460be8 <_DYNAMIC+0x668>
  40bf0c:	e9 a5 00 00 00       	jmp    40bfb6 <pt_str_from_int+0x646>
  40bf11:	74 38                	je     40bf4b <pt_str_from_int+0x5db>
  40bf13:	4c 8b 7c 24 28       	mov    0x28(%rsp),%r15
  40bf18:	ff 15 02 4b 05 00    	call   *0x54b02(%rip)        # 460a20 <_DYNAMIC+0x4a0>
  40bf1e:	41 bc 01 00 00 00    	mov    $0x1,%r12d
  40bf24:	be 01 00 00 00       	mov    $0x1,%esi
  40bf29:	48 89 df             	mov    %rbx,%rdi
  40bf2c:	ff 15 2e 4b 05 00    	call   *0x54b2e(%rip)        # 460a60 <_DYNAMIC+0x4e0>
  40bf32:	48 85 c0             	test   %rax,%rax
  40bf35:	74 c9                	je     40bf00 <pt_str_from_int+0x590>
  40bf37:	49 89 c6             	mov    %rax,%r14
  40bf3a:	48 89 c7             	mov    %rax,%rdi
  40bf3d:	4c 89 fe             	mov    %r15,%rsi
  40bf40:	48 89 da             	mov    %rbx,%rdx
  40bf43:	ff 15 37 4f 05 00    	call   *0x54f37(%rip)        # 460e80 <_DYNAMIC+0x900>
  40bf49:	eb 06                	jmp    40bf51 <pt_str_from_int+0x5e1>
  40bf4b:	41 be 01 00 00 00    	mov    $0x1,%r14d
  40bf51:	ff 15 c9 4a 05 00    	call   *0x54ac9(%rip)        # 460a20 <_DYNAMIC+0x4a0>
  40bf57:	bf 20 00 00 00       	mov    $0x20,%edi
  40bf5c:	be 08 00 00 00       	mov    $0x8,%esi
  40bf61:	ff 15 f9 4a 05 00    	call   *0x54af9(%rip)        # 460a60 <_DYNAMIC+0x4e0>
  40bf67:	48 85 c0             	test   %rax,%rax
  40bf6a:	74 3a                	je     40bfa6 <pt_str_from_int+0x636>
  40bf6c:	4c 89 30             	mov    %r14,(%rax)
  40bf6f:	48 89 58 08          	mov    %rbx,0x8(%rax)
  40bf73:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
  40bf7a:	00 
  40bf7b:	48 89 58 18          	mov    %rbx,0x18(%rax)
  40bf7f:	48 83 c4 38          	add    $0x38,%rsp
  40bf83:	5b                   	pop    %rbx
  40bf84:	41 5c                	pop    %r12
  40bf86:	41 5e                	pop    %r14
  40bf88:	41 5f                	pop    %r15
  40bf8a:	c3                   	ret
  40bf8b:	48 8d 3d 5e 3d 04 00 	lea    0x43d5e(%rip),%rdi        # 44fcf0 <anon.13df560d6b83da2c10641ad9b2156d4a.31.llvm.366253514071984098+0x461>
  40bf92:	48 8d 15 5f 24 05 00 	lea    0x5245f(%rip),%rdx        # 45e3f8 <__do_global_dtors_aux_fini_array_entry+0x118>
  40bf99:	be 97 00 00 00       	mov    $0x97,%esi
  40bf9e:	ff 15 64 4b 05 00    	call   *0x54b64(%rip)        # 460b08 <_DYNAMIC+0x588>
  40bfa4:	eb 10                	jmp    40bfb6 <pt_str_from_int+0x646>
  40bfa6:	bf 08 00 00 00       	mov    $0x8,%edi
  40bfab:	be 20 00 00 00       	mov    $0x20,%esi
  40bfb0:	ff 15 a2 4c 05 00    	call   *0x54ca2(%rip)        # 460c58 <_DYNAMIC+0x6d8>
  40bfb6:	0f 0b                	ud2
  40bfb8:	48 85 db             	test   %rbx,%rbx
  40bfbb:	74 17                	je     40bfd4 <pt_str_from_int+0x664>
  40bfbd:	ba 01 00 00 00       	mov    $0x1,%edx
  40bfc2:	4c 89 f7             	mov    %r14,%rdi
  40bfc5:	48 89 de             	mov    %rbx,%rsi
  40bfc8:	ff 15 5a 4f 05 00    	call   *0x54f5a(%rip)        # 460f28 <_DYNAMIC+0x9a8>
  40bfce:	ff 15 d4 48 05 00    	call   *0x548d4(%rip)        # 4608a8 <_DYNAMIC+0x328>
  40bfd4:	ff 15 ce 48 05 00    	call   *0x548ce(%rip)        # 4608a8 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card817/probe-scoped/counts:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .text:

000000000040c270 <pt_str_new>:
  40c270:	41 57                	push   %r15
  40c272:	41 56                	push   %r14
  40c274:	53                   	push   %rbx
  40c275:	48 83 ec 20          	sub    $0x20,%rsp
  40c279:	48 89 f2             	mov    %rsi,%rdx
  40c27c:	48 89 fe             	mov    %rdi,%rsi
  40c27f:	48 8d 7c 24 08       	lea    0x8(%rsp),%rdi
  40c284:	ff 15 4e 4c 05 00    	call   *0x54c4e(%rip)        # 460ed8 <_DYNAMIC+0x958>
  40c28a:	48 83 7c 24 08 ff    	cmpq   $0xffffffffffffffff,0x8(%rsp)
  40c290:	74 25                	je     40c2b7 <pt_str_new+0x47>
  40c292:	48 8d 7c 24 08       	lea    0x8(%rsp),%rdi
  40c297:	ff 15 5b 4b 05 00    	call   *0x54b5b(%rip)        # 460df8 <_DYNAMIC+0x878>
  40c29d:	49 89 c6             	mov    %rax,%r14
  40c2a0:	48 89 d3             	mov    %rdx,%rbx
  40c2a3:	48 83 fb 20          	cmp    $0x20,%rbx
  40c2a7:	73 64                	jae    40c30d <pt_str_new+0x9d>
  40c2a9:	4c 89 f7             	mov    %r14,%rdi
  40c2ac:	48 89 de             	mov    %rbx,%rsi
  40c2af:	ff 15 53 4a 05 00    	call   *0x54a53(%rip)        # 460d08 <_DYNAMIC+0x788>
  40c2b5:	eb 62                	jmp    40c319 <pt_str_new+0xa9>
  40c2b7:	4c 8b 7c 24 10       	mov    0x10(%rsp),%r15
  40c2bc:	48 8b 5c 24 18       	mov    0x18(%rsp),%rbx
  40c2c1:	48 85 db             	test   %rbx,%rbx
  40c2c4:	74 2c                	je     40c2f2 <pt_str_new+0x82>
  40c2c6:	ff 15 54 47 05 00    	call   *0x54754(%rip)        # 460a20 <_DYNAMIC+0x4a0>
  40c2cc:	be 01 00 00 00       	mov    $0x1,%esi
  40c2d1:	48 89 df             	mov    %rbx,%rdi
  40c2d4:	ff 15 86 47 05 00    	call   *0x54786(%rip)        # 460a60 <_DYNAMIC+0x4e0>
  40c2da:	49 89 c6             	mov    %rax,%r14
  40c2dd:	48 85 c0             	test   %rax,%rax
  40c2e0:	75 16                	jne    40c2f8 <pt_str_new+0x88>
  40c2e2:	bf 01 00 00 00       	mov    $0x1,%edi
  40c2e7:	48 89 de             	mov    %rbx,%rsi
  40c2ea:	ff 15 68 49 05 00    	call   *0x54968(%rip)        # 460c58 <_DYNAMIC+0x6d8>
  40c2f0:	eb 72                	jmp    40c364 <pt_str_new+0xf4>
  40c2f2:	41 be 01 00 00 00    	mov    $0x1,%r14d
  40c2f8:	4c 89 f7             	mov    %r14,%rdi
  40c2fb:	4c 89 fe             	mov    %r15,%rsi
  40c2fe:	48 89 da             	mov    %rbx,%rdx
  40c301:	ff 15 79 4b 05 00    	call   *0x54b79(%rip)        # 460e80 <_DYNAMIC+0x900>
  40c307:	48 83 fb 20          	cmp    $0x20,%rbx
  40c30b:	72 9c                	jb     40c2a9 <pt_str_new+0x39>
  40c30d:	4c 89 f7             	mov    %r14,%rdi
  40c310:	48 89 de             	mov    %rbx,%rsi
  40c313:	ff 15 af 4b 05 00    	call   *0x54baf(%rip)        # 460ec8 <_DYNAMIC+0x948>
  40c319:	49 89 c7             	mov    %rax,%r15
  40c31c:	ff 15 fe 46 05 00    	call   *0x546fe(%rip)        # 460a20 <_DYNAMIC+0x4a0>
  40c322:	bf 20 00 00 00       	mov    $0x20,%edi
  40c327:	be 08 00 00 00       	mov    $0x8,%esi
  40c32c:	ff 15 2e 47 05 00    	call   *0x5472e(%rip)        # 460a60 <_DYNAMIC+0x4e0>
  40c332:	48 85 c0             	test   %rax,%rax
  40c335:	74 1d                	je     40c354 <pt_str_new+0xe4>
  40c337:	4c 89 30             	mov    %r14,(%rax)
  40c33a:	48 89 58 08          	mov    %rbx,0x8(%rax)
  40c33e:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
  40c345:	00 
  40c346:	4c 89 78 18          	mov    %r15,0x18(%rax)
  40c34a:	48 83 c4 20          	add    $0x20,%rsp
  40c34e:	5b                   	pop    %rbx
  40c34f:	41 5e                	pop    %r14
  40c351:	41 5f                	pop    %r15
  40c353:	c3                   	ret
  40c354:	bf 08 00 00 00       	mov    $0x8,%edi
  40c359:	be 20 00 00 00       	mov    $0x20,%esi
  40c35e:	ff 15 f4 48 05 00    	call   *0x548f4(%rip)        # 460c58 <_DYNAMIC+0x6d8>
  40c364:	0f 0b                	ud2
  40c366:	eb 19                	jmp    40c381 <pt_str_new+0x111>
  40c368:	eb 0c                	jmp    40c376 <pt_str_new+0x106>
  40c36a:	ff 15 38 45 05 00    	call   *0x54538(%rip)        # 4608a8 <_DYNAMIC+0x328>
  40c370:	ff 15 32 45 05 00    	call   *0x54532(%rip)        # 4608a8 <_DYNAMIC+0x328>
  40c376:	48 85 db             	test   %rbx,%rbx
  40c379:	75 06                	jne    40c381 <pt_str_new+0x111>
  40c37b:	ff 15 27 45 05 00    	call   *0x54527(%rip)        # 4608a8 <_DYNAMIC+0x328>
  40c381:	ba 01 00 00 00       	mov    $0x1,%edx
  40c386:	4c 89 f7             	mov    %r14,%rdi
  40c389:	48 89 de             	mov    %rbx,%rsi
  40c38c:	ff 15 96 4b 05 00    	call   *0x54b96(%rip)        # 460f28 <_DYNAMIC+0x9a8>
  40c392:	ff 15 10 45 05 00    	call   *0x54510(%rip)        # 4608a8 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card817/probe-scoped/counts:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .text:

000000000040bfe0 <pt_str_index>:
  40bfe0:	41 57                	push   %r15
  40bfe2:	41 56                	push   %r14
  40bfe4:	41 54                	push   %r12
  40bfe6:	53                   	push   %rbx
  40bfe7:	50                   	push   %rax
  40bfe8:	48 8b 47 18          	mov    0x18(%rdi),%rax
  40bfec:	48 85 c0             	test   %rax,%rax
  40bfef:	0f 88 21 01 00 00    	js     40c116 <pt_str_index+0x136>
  40bff5:	49 89 f4             	mov    %rsi,%r12
  40bff8:	49 c1 fc 3f          	sar    $0x3f,%r12
  40bffc:	49 21 c4             	and    %rax,%r12
  40bfff:	49 01 f4             	add    %rsi,%r12
  40c002:	0f 99 c1             	setns  %cl
  40c005:	49 39 c4             	cmp    %rax,%r12
  40c008:	0f 92 c2             	setb   %dl
  40c00b:	20 ca                	and    %cl,%dl
  40c00d:	80 fa 01             	cmp    $0x1,%dl
  40c010:	0f 85 00 01 00 00    	jne    40c116 <pt_str_index+0x136>
  40c016:	48 8b 1f             	mov    (%rdi),%rbx
  40c019:	48 8b 77 08          	mov    0x8(%rdi),%rsi
  40c01d:	48 39 f0             	cmp    %rsi,%rax
  40c020:	75 33                	jne    40c055 <pt_str_index+0x75>
  40c022:	4d 85 e4             	test   %r12,%r12
  40c025:	74 0b                	je     40c032 <pt_str_index+0x52>
  40c027:	42 80 3c 23 bf       	cmpb   $0xbf,(%rbx,%r12,1)
  40c02c:	0f 8e e4 00 00 00    	jle    40c116 <pt_str_index+0x136>
  40c032:	49 8d 4c 24 01       	lea    0x1(%r12),%rcx
  40c037:	41 be 01 00 00 00    	mov    $0x1,%r14d
  40c03d:	48 39 c1             	cmp    %rax,%rcx
  40c040:	0f 83 ca 00 00 00    	jae    40c110 <pt_str_index+0x130>
  40c046:	80 3c 0b bf          	cmpb   $0xbf,(%rbx,%rcx,1)
  40c04a:	0f 8f 4a 01 00 00    	jg     40c19a <pt_str_index+0x1ba>
  40c050:	e9 c1 00 00 00       	jmp    40c116 <pt_str_index+0x136>
  40c055:	48 8d 0c 33          	lea    (%rbx,%rsi,1),%rcx
  40c059:	4d 85 e4             	test   %r12,%r12
  40c05c:	74 56                	je     40c0b4 <pt_str_index+0xd4>
  40c05e:	31 c0                	xor    %eax,%eax
  40c060:	31 d2                	xor    %edx,%edx
  40c062:	49 89 d8             	mov    %rbx,%r8
  40c065:	eb 1e                	jmp    40c085 <pt_str_index+0xa5>
  40c067:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  40c06e:	00 00 
  40c070:	49 8d 78 01          	lea    0x1(%r8),%rdi
  40c074:	49 89 f9             	mov    %rdi,%r9
  40c077:	4d 29 c1             	sub    %r8,%r9
  40c07a:	4c 01 ca             	add    %r9,%rdx
  40c07d:	49 89 f8             	mov    %rdi,%r8
  40c080:	49 ff cc             	dec    %r12
  40c083:	74 34                	je     40c0b9 <pt_str_index+0xd9>
  40c085:	49 39 c8             	cmp    %rcx,%r8
  40c088:	0f 84 8a 00 00 00    	je     40c118 <pt_str_index+0x138>
  40c08e:	41 0f b6 38          	movzbl (%r8),%edi
  40c092:	40 84 ff             	test   %dil,%dil
  40c095:	79 d9                	jns    40c070 <pt_str_index+0x90>
  40c097:	40 80 ff e0          	cmp    $0xe0,%dil
  40c09b:	72 11                	jb     40c0ae <pt_str_index+0xce>
  40c09d:	40 80 ff f0          	cmp    $0xf0,%dil
  40c0a1:	4c 89 c7             	mov    %r8,%rdi
  40c0a4:	48 83 df ff          	sbb    $0xffffffffffffffff,%rdi
  40c0a8:	48 83 c7 03          	add    $0x3,%rdi
  40c0ac:	eb c6                	jmp    40c074 <pt_str_index+0x94>
  40c0ae:	49 8d 78 02          	lea    0x2(%r8),%rdi
  40c0b2:	eb c0                	jmp    40c074 <pt_str_index+0x94>
  40c0b4:	31 d2                	xor    %edx,%edx
  40c0b6:	48 89 df             	mov    %rbx,%rdi
  40c0b9:	48 39 cf             	cmp    %rcx,%rdi
  40c0bc:	74 58                	je     40c116 <pt_str_index+0x136>
  40c0be:	44 0f b6 07          	movzbl (%rdi),%r8d
  40c0c2:	41 be 01 00 00 00    	mov    $0x1,%r14d
  40c0c8:	45 84 c0             	test   %r8b,%r8b
  40c0cb:	0f 89 83 00 00 00    	jns    40c154 <pt_str_index+0x174>
  40c0d1:	44 89 c0             	mov    %r8d,%eax
  40c0d4:	83 e0 1f             	and    $0x1f,%eax
  40c0d7:	44 0f b6 4f 01       	movzbl 0x1(%rdi),%r9d
  40c0dc:	41 83 e1 3f          	and    $0x3f,%r9d
  40c0e0:	41 80 f8 e0          	cmp    $0xe0,%r8b
  40c0e4:	72 3e                	jb     40c124 <pt_str_index+0x144>
  40c0e6:	0f b6 4f 02          	movzbl 0x2(%rdi),%ecx
  40c0ea:	41 c1 e1 06          	shl    $0x6,%r9d
  40c0ee:	83 e1 3f             	and    $0x3f,%ecx
  40c0f1:	44 09 c9             	or     %r9d,%ecx
  40c0f4:	41 80 f8 f0          	cmp    $0xf0,%r8b
  40c0f8:	72 32                	jb     40c12c <pt_str_index+0x14c>
  40c0fa:	0f b6 7f 03          	movzbl 0x3(%rdi),%edi
  40c0fe:	83 e0 07             	and    $0x7,%eax
  40c101:	c1 e0 12             	shl    $0x12,%eax
  40c104:	c1 e1 06             	shl    $0x6,%ecx
  40c107:	83 e7 3f             	and    $0x3f,%edi
  40c10a:	09 cf                	or     %ecx,%edi
  40c10c:	09 f8                	or     %edi,%eax
  40c10e:	eb 21                	jmp    40c131 <pt_str_index+0x151>
  40c110:	0f 84 84 00 00 00    	je     40c19a <pt_str_index+0x1ba>
  40c116:	31 c0                	xor    %eax,%eax
  40c118:	48 83 c4 08          	add    $0x8,%rsp
  40c11c:	5b                   	pop    %rbx
  40c11d:	41 5c                	pop    %r12
  40c11f:	41 5e                	pop    %r14
  40c121:	41 5f                	pop    %r15
  40c123:	c3                   	ret
  40c124:	c1 e0 06             	shl    $0x6,%eax
  40c127:	44 09 c8             	or     %r9d,%eax
  40c12a:	eb 05                	jmp    40c131 <pt_str_index+0x151>
  40c12c:	c1 e0 0c             	shl    $0xc,%eax
  40c12f:	09 c8                	or     %ecx,%eax
  40c131:	3d 80 00 00 00       	cmp    $0x80,%eax
  40c136:	72 1c                	jb     40c154 <pt_str_index+0x174>
  40c138:	41 be 02 00 00 00    	mov    $0x2,%r14d
  40c13e:	3d 00 08 00 00       	cmp    $0x800,%eax
  40c143:	72 0f                	jb     40c154 <pt_str_index+0x174>
  40c145:	3d 00 00 01 00       	cmp    $0x10000,%eax
  40c14a:	41 be 04 00 00 00    	mov    $0x4,%r14d
  40c150:	49 83 de 00          	sbb    $0x0,%r14
  40c154:	4c 89 f1             	mov    %r14,%rcx
  40c157:	48 01 d1             	add    %rdx,%rcx
  40c15a:	72 24                	jb     40c180 <pt_str_index+0x1a0>
  40c15c:	48 85 d2             	test   %rdx,%rdx
  40c15f:	74 0f                	je     40c170 <pt_str_index+0x190>
  40c161:	48 39 f2             	cmp    %rsi,%rdx
  40c164:	73 08                	jae    40c16e <pt_str_index+0x18e>
  40c166:	80 3c 13 bf          	cmpb   $0xbf,(%rbx,%rdx,1)
  40c16a:	7f 04                	jg     40c170 <pt_str_index+0x190>
  40c16c:	eb 12                	jmp    40c180 <pt_str_index+0x1a0>
  40c16e:	75 10                	jne    40c180 <pt_str_index+0x1a0>
  40c170:	48 85 c9             	test   %rcx,%rcx
  40c173:	74 22                	je     40c197 <pt_str_index+0x1b7>
  40c175:	48 39 f1             	cmp    %rsi,%rcx
  40c178:	73 1b                	jae    40c195 <pt_str_index+0x1b5>
  40c17a:	80 3c 0b bf          	cmpb   $0xbf,(%rbx,%rcx,1)
  40c17e:	7f 17                	jg     40c197 <pt_str_index+0x1b7>
  40c180:	4c 8d 05 59 22 05 00 	lea    0x52259(%rip),%r8        # 45e3e0 <__do_global_dtors_aux_fini_array_entry+0x100>
  40c187:	48 89 df             	mov    %rbx,%rdi
  40c18a:	ff 15 20 46 05 00    	call   *0x54620(%rip)        # 4607b0 <_DYNAMIC+0x230>
  40c190:	e9 95 00 00 00       	jmp    40c22a <pt_str_index+0x24a>
  40c195:	75 e9                	jne    40c180 <pt_str_index+0x1a0>
  40c197:	49 89 d4             	mov    %rdx,%r12
  40c19a:	ff 15 80 48 05 00    	call   *0x54880(%rip)        # 460a20 <_DYNAMIC+0x4a0>
  40c1a0:	be 01 00 00 00       	mov    $0x1,%esi
  40c1a5:	4c 89 f7             	mov    %r14,%rdi
  40c1a8:	ff 15 b2 48 05 00    	call   *0x548b2(%rip)        # 460a60 <_DYNAMIC+0x4e0>
  40c1ae:	48 85 c0             	test   %rax,%rax
  40c1b1:	74 69                	je     40c21c <pt_str_index+0x23c>
  40c1b3:	49 89 c7             	mov    %rax,%r15
  40c1b6:	4c 01 e3             	add    %r12,%rbx
  40c1b9:	48 89 c7             	mov    %rax,%rdi
  40c1bc:	48 89 de             	mov    %rbx,%rsi
  40c1bf:	4c 89 f2             	mov    %r14,%rdx
  40c1c2:	ff 15 b8 4c 05 00    	call   *0x54cb8(%rip)        # 460e80 <_DYNAMIC+0x900>
  40c1c8:	4c 89 ff             	mov    %r15,%rdi
  40c1cb:	4c 89 f6             	mov    %r14,%rsi
  40c1ce:	ff 15 34 4b 05 00    	call   *0x54b34(%rip)        # 460d08 <_DYNAMIC+0x788>
  40c1d4:	48 89 c3             	mov    %rax,%rbx
  40c1d7:	ff 15 43 48 05 00    	call   *0x54843(%rip)        # 460a20 <_DYNAMIC+0x4a0>
  40c1dd:	bf 20 00 00 00       	mov    $0x20,%edi
  40c1e2:	be 08 00 00 00       	mov    $0x8,%esi
  40c1e7:	ff 15 73 48 05 00    	call   *0x54873(%rip)        # 460a60 <_DYNAMIC+0x4e0>
  40c1ed:	48 85 c0             	test   %rax,%rax
  40c1f0:	74 18                	je     40c20a <pt_str_index+0x22a>
  40c1f2:	4c 89 38             	mov    %r15,(%rax)
  40c1f5:	4c 89 70 08          	mov    %r14,0x8(%rax)
  40c1f9:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
  40c200:	00 
  40c201:	48 89 58 18          	mov    %rbx,0x18(%rax)
  40c205:	e9 0e ff ff ff       	jmp    40c118 <pt_str_index+0x138>
  40c20a:	bf 08 00 00 00       	mov    $0x8,%edi
  40c20f:	be 20 00 00 00       	mov    $0x20,%esi
  40c214:	ff 15 3e 4a 05 00    	call   *0x54a3e(%rip)        # 460c58 <_DYNAMIC+0x6d8>
  40c21a:	eb 0e                	jmp    40c22a <pt_str_index+0x24a>
  40c21c:	bf 01 00 00 00       	mov    $0x1,%edi
  40c221:	4c 89 f6             	mov    %r14,%rsi
  40c224:	ff 15 2e 4a 05 00    	call   *0x54a2e(%rip)        # 460c58 <_DYNAMIC+0x6d8>
  40c22a:	0f 0b                	ud2
  40c22c:	eb 06                	jmp    40c234 <pt_str_index+0x254>
  40c22e:	ff 15 74 46 05 00    	call   *0x54674(%rip)        # 4608a8 <_DYNAMIC+0x328>
  40c234:	ba 01 00 00 00       	mov    $0x1,%edx
  40c239:	4c 89 ff             	mov    %r15,%rdi
  40c23c:	4c 89 f6             	mov    %r14,%rsi
  40c23f:	ff 15 e3 4c 05 00    	call   *0x54ce3(%rip)        # 460f28 <_DYNAMIC+0x9a8>
  40c245:	ff 15 5d 46 05 00    	call   *0x5465d(%rip)        # 4608a8 <_DYNAMIC+0x328>

Disassembly of section .fini:
