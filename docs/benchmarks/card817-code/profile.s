
/workspace/card817/probe-scoped/profile:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .text:

00000000004077c8 <pythrust_main>:
  4077c8:	55                   	push   %rbp
  4077c9:	48 89 e5             	mov    %rsp,%rbp
  4077cc:	48 81 ec 80 00 00 00 	sub    $0x80,%rsp
  4077d3:	48 89 5c 24 50       	mov    %rbx,0x50(%rsp)
  4077d8:	4c 89 64 24 58       	mov    %r12,0x58(%rsp)
  4077dd:	4c 89 6c 24 60       	mov    %r13,0x60(%rsp)
  4077e2:	4c 89 74 24 68       	mov    %r14,0x68(%rsp)
  4077e7:	4c 89 7c 24 70       	mov    %r15,0x70(%rsp)
  4077ec:	48 8d 3d 0d 38 04 00 	lea    0x4380d(%rip),%rdi        # 44b000 <.Ldata1>
  4077f3:	48 33 db             	xor    %rbx,%rbx
  4077f6:	48 8d 05 c3 49 00 00 	lea    0x49c3(%rip),%rax        # 40c1c0 <pt_str_new>
  4077fd:	48 89 de             	mov    %rbx,%rsi
  407800:	ff d0                	call   *%rax
  407802:	4c 8d 25 f7 37 04 00 	lea    0x437f7(%rip),%r12        # 44b000 <.Ldata1>
  407809:	4c 8d 2d f1 37 04 00 	lea    0x437f1(%rip),%r13        # 44b001 <.Ldata2>
  407810:	49 89 c7             	mov    %rax,%r15
  407813:	48 89 5c 24 10       	mov    %rbx,0x10(%rsp)
  407818:	48 89 5c 24 18       	mov    %rbx,0x18(%rsp)
  40781d:	48 81 fb 40 42 0f 00 	cmp    $0xf4240,%rbx
  407824:	0f 8d 40 02 00 00    	jge    407a6a <pythrust_main+0x2a2>
  40782a:	41 be 01 00 00 00    	mov    $0x1,%r14d
  407830:	48 8d 05 89 49 00 00 	lea    0x4989(%rip),%rax        # 40c1c0 <pt_str_new>
  407837:	4c 89 f6             	mov    %r14,%rsi
  40783a:	4c 89 e7             	mov    %r12,%rdi
  40783d:	ff d0                	call   *%rax
  40783f:	48 89 44 24 48       	mov    %rax,0x48(%rsp)
  407844:	48 8d 05 75 40 00 00 	lea    0x4075(%rip),%rax        # 40b8c0 <pt_str_from_int>
  40784b:	48 89 df             	mov    %rbx,%rdi
  40784e:	ff d0                	call   *%rax
  407850:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
  407855:	48 8d 05 a4 3e 00 00 	lea    0x3ea4(%rip),%rax        # 40b700 <pt_str_concat>
  40785c:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
  407861:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
  407866:	ff d0                	call   *%rax
  407868:	48 89 44 24 38       	mov    %rax,0x38(%rsp)
  40786d:	48 8d 05 4c 49 00 00 	lea    0x494c(%rip),%rax        # 40c1c0 <pt_str_new>
  407874:	4c 89 f6             	mov    %r14,%rsi
  407877:	4c 89 ef             	mov    %r13,%rdi
  40787a:	ff d0                	call   *%rax
  40787c:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
  407881:	48 8d 0d 78 3e 00 00 	lea    0x3e78(%rip),%rcx        # 40b700 <pt_str_concat>
  407888:	48 8b 74 24 30       	mov    0x30(%rsp),%rsi
  40788d:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
  407892:	ff d1                	call   *%rcx
  407894:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
  407899:	48 89 d8             	mov    %rbx,%rax
  40789c:	48 f7 2d f5 02 00 00 	imulq  0x2f5(%rip)        # 407b98 <pythrust_main+0x3d0>
  4078a3:	48 8d 04 1a          	lea    (%rdx,%rbx,1),%rax
  4078a7:	48 c1 f8 06          	sar    $0x6,%rax
  4078ab:	48 89 c1             	mov    %rax,%rcx
  4078ae:	48 c1 e9 3f          	shr    $0x3f,%rcx
  4078b2:	48 03 c1             	add    %rcx,%rax
  4078b5:	48 6b c8 61          	imul   $0x61,%rax,%rcx
  4078b9:	48 89 d8             	mov    %rbx,%rax
  4078bc:	48 2b c1             	sub    %rcx,%rax
  4078bf:	48 3b d9             	cmp    %rcx,%rbx
  4078c2:	0f 95 c1             	setne  %cl
  4078c5:	48 89 c6             	mov    %rax,%rsi
  4078c8:	48 83 f6 61          	xor    $0x61,%rsi
  4078cc:	48 33 d2             	xor    %rdx,%rdx
  4078cf:	48 c1 ee 3f          	shr    $0x3f,%rsi
  4078d3:	40 84 ce             	test   %cl,%sil
  4078d6:	48 0f 45 15 c2 02 00 	cmovne 0x2c2(%rip),%rdx        # 407ba0 <pythrust_main+0x3d8>
  4078dd:	00 
  4078de:	48 8d 3c 10          	lea    (%rax,%rdx,1),%rdi
  4078e2:	48 8d 05 d7 3f 00 00 	lea    0x3fd7(%rip),%rax        # 40b8c0 <pt_str_from_int>
  4078e9:	ff d0                	call   *%rax
  4078eb:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
  4078f0:	48 8d 0d 09 3e 00 00 	lea    0x3e09(%rip),%rcx        # 40b700 <pt_str_concat>
  4078f7:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
  4078fc:	48 8b 7c 24 20       	mov    0x20(%rsp),%rdi
  407901:	ff d1                	call   *%rcx
  407903:	49 89 c6             	mov    %rax,%r14
  407906:	48 8d 05 33 3f 00 00 	lea    0x3f33(%rip),%rax        # 40b840 <pt_str_dec>
  40790d:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
  407912:	ff d0                	call   *%rax
  407914:	48 8d 05 25 3f 00 00 	lea    0x3f25(%rip),%rax        # 40b840 <pt_str_dec>
  40791b:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
  407920:	ff d0                	call   *%rax
  407922:	48 8d 05 17 3f 00 00 	lea    0x3f17(%rip),%rax        # 40b840 <pt_str_dec>
  407929:	48 8b 7c 24 40       	mov    0x40(%rsp),%rdi
  40792e:	ff d0                	call   *%rax
  407930:	48 8d 05 09 3f 00 00 	lea    0x3f09(%rip),%rax        # 40b840 <pt_str_dec>
  407937:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
  40793c:	ff d0                	call   *%rax
  40793e:	48 8d 05 fb 3e 00 00 	lea    0x3efb(%rip),%rax        # 40b840 <pt_str_dec>
  407945:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
  40794a:	ff d0                	call   *%rax
  40794c:	48 8d 0d ed 3e 00 00 	lea    0x3eed(%rip),%rcx        # 40b840 <pt_str_dec>
  407953:	48 8b 7c 24 20       	mov    0x20(%rsp),%rdi
  407958:	ff d1                	call   *%rcx
  40795a:	48 8d 15 df 3e 00 00 	lea    0x3edf(%rip),%rdx        # 40b840 <pt_str_dec>
  407961:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
  407966:	ff d2                	call   *%rdx
  407968:	48 8d 35 31 48 00 00 	lea    0x4831(%rip),%rsi        # 40c1a0 <pt_str_len>
  40796f:	4c 89 f7             	mov    %r14,%rdi
  407972:	ff d6                	call   *%rsi
  407974:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
  407979:	48 03 f8             	add    %rax,%rdi
  40797c:	48 89 7c 24 10       	mov    %rdi,0x10(%rsp)
  407981:	40 0f 90 c6          	seto   %sil
  407985:	40 84 f6             	test   %sil,%sil
  407988:	0f 85 ef 01 00 00    	jne    407b7d <pythrust_main+0x3b5>
  40798e:	48 89 d8             	mov    %rbx,%rax
  407991:	48 f7 2d 10 02 00 00 	imulq  0x210(%rip)        # 407ba8 <pythrust_main+0x3e0>
  407998:	48 8d 34 1a          	lea    (%rdx,%rbx,1),%rsi
  40799c:	48 c1 fe 05          	sar    $0x5,%rsi
  4079a0:	48 89 f7             	mov    %rsi,%rdi
  4079a3:	48 c1 ef 3f          	shr    $0x3f,%rdi
  4079a7:	48 03 f7             	add    %rdi,%rsi
  4079aa:	48 6b fe 32          	imul   $0x32,%rsi,%rdi
  4079ae:	48 89 de             	mov    %rbx,%rsi
  4079b1:	48 2b f7             	sub    %rdi,%rsi
  4079b4:	48 3b df             	cmp    %rdi,%rbx
  4079b7:	40 0f 95 c7          	setne  %dil
  4079bb:	49 89 f1             	mov    %rsi,%r9
  4079be:	49 83 f1 32          	xor    $0x32,%r9
  4079c2:	4d 33 c0             	xor    %r8,%r8
  4079c5:	49 c1 e9 3f          	shr    $0x3f,%r9
  4079c9:	41 84 f9             	test   %dil,%r9b
  4079cc:	4c 0f 45 05 dc 01 00 	cmovne 0x1dc(%rip),%r8        # 407bb0 <pythrust_main+0x3e8>
  4079d3:	00 
  4079d4:	4c 03 c6             	add    %rsi,%r8
  4079d7:	4d 85 c0             	test   %r8,%r8
  4079da:	0f 85 55 00 00 00    	jne    407a35 <pythrust_main+0x26d>
  4079e0:	48 33 f6             	xor    %rsi,%rsi
  4079e3:	48 8d 05 46 45 00 00 	lea    0x4546(%rip),%rax        # 40bf30 <pt_str_index>
  4079ea:	4c 89 f7             	mov    %r14,%rdi
  4079ed:	ff d0                	call   *%rax
  4079ef:	48 89 c2             	mov    %rax,%rdx
  4079f2:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
  4079f7:	48 85 d0             	test   %rdx,%rax
  4079fa:	0f 84 12 01 00 00    	je     407b12 <pythrust_main+0x34a>
  407a00:	48 8d 0d f9 3c 00 00 	lea    0x3cf9(%rip),%rcx        # 40b700 <pt_str_concat>
  407a07:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
  407a0c:	4c 89 ff             	mov    %r15,%rdi
  407a0f:	ff d1                	call   *%rcx
  407a11:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
  407a16:	48 8d 05 23 3e 00 00 	lea    0x3e23(%rip),%rax        # 40b840 <pt_str_dec>
  407a1d:	4c 89 ff             	mov    %r15,%rdi
  407a20:	ff d0                	call   *%rax
  407a22:	48 8d 05 17 3e 00 00 	lea    0x3e17(%rip),%rax        # 40b840 <pt_str_dec>
  407a29:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
  407a2e:	ff d0                	call   *%rax
  407a30:	4c 8b 7c 24 20       	mov    0x20(%rsp),%r15
  407a35:	48 89 d8             	mov    %rbx,%rax
  407a38:	48 83 c0 01          	add    $0x1,%rax
  407a3c:	0f 90 c0             	seto   %al
  407a3f:	84 c0                	test   %al,%al
  407a41:	0f 84 0a 00 00 00    	je     407a51 <pythrust_main+0x289>
  407a47:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
  407a4c:	e9 19 00 00 00       	jmp    407a6a <pythrust_main+0x2a2>
  407a51:	48 83 c3 01          	add    $0x1,%rbx
  407a55:	0f 90 c0             	seto   %al
  407a58:	84 c0                	test   %al,%al
  407a5a:	0f 85 9b 00 00 00    	jne    407afb <pythrust_main+0x333>
  407a60:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
  407a65:	e9 b3 fd ff ff       	jmp    40781d <pythrust_main+0x55>
  407a6a:	48 8d 05 2f 47 00 00 	lea    0x472f(%rip),%rax        # 40c1a0 <pt_str_len>
  407a71:	4c 89 ff             	mov    %r15,%rdi
  407a74:	ff d0                	call   *%rax
  407a76:	48 89 c3             	mov    %rax,%rbx
  407a79:	48 8d 05 c0 6d 00 00 	lea    0x6dc0(%rip),%rax        # 40e840 <pt_write_int>
  407a80:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
  407a85:	ff d0                	call   *%rax
  407a87:	b8 20 00 00 00       	mov    $0x20,%eax
  407a8c:	48 0f b6 f8          	movzbq %al,%rdi
  407a90:	48 8d 05 79 6d 00 00 	lea    0x6d79(%rip),%rax        # 40e810 <pt_write_byte>
  407a97:	ff d0                	call   *%rax
  407a99:	48 8d 05 a0 6d 00 00 	lea    0x6da0(%rip),%rax        # 40e840 <pt_write_int>
  407aa0:	48 89 df             	mov    %rbx,%rdi
  407aa3:	ff d0                	call   *%rax
  407aa5:	b8 0a 00 00 00       	mov    $0xa,%eax
  407aaa:	48 0f b6 f8          	movzbq %al,%rdi
  407aae:	48 8d 05 5b 6d 00 00 	lea    0x6d5b(%rip),%rax        # 40e810 <pt_write_byte>
  407ab5:	ff d0                	call   *%rax
  407ab7:	48 8d 05 82 3d 00 00 	lea    0x3d82(%rip),%rax        # 40b840 <pt_str_dec>
  407abe:	4c 89 ff             	mov    %r15,%rdi
  407ac1:	ff d0                	call   *%rax
  407ac3:	48 8d 05 76 3d 00 00 	lea    0x3d76(%rip),%rax        # 40b840 <pt_str_dec>
  407aca:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
  407acf:	ff d0                	call   *%rax
  407ad1:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
  407ad6:	4c 8b 64 24 58       	mov    0x58(%rsp),%r12
  407adb:	4c 8b 6c 24 60       	mov    0x60(%rsp),%r13
  407ae0:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
  407ae5:	4c 8b 7c 24 70       	mov    0x70(%rsp),%r15
  407aea:	48 81 c4 80 00 00 00 	add    $0x80,%rsp
  407af1:	48 89 ec             	mov    %rbp,%rsp
  407af4:	5d                   	pop    %rbp
  407af5:	c3                   	ret
  407af6:	e9 53 00 00 00       	jmp    407b4e <pythrust_main+0x386>
  407afb:	48 8d 3d b4 36 04 00 	lea    0x436b4(%rip),%rdi        # 44b1b6 <.Ldata9>
  407b02:	be 96 00 00 00       	mov    $0x96,%esi
  407b07:	48 8d 05 a2 6c 00 00 	lea    0x6ca2(%rip),%rax        # 40e7b0 <pt_fail>
  407b0e:	ff d0                	call   *%rax
  407b10:	0f 0b                	ud2
  407b12:	b8 01 00 00 00       	mov    $0x1,%eax
  407b17:	48 8d 35 c3 37 04 00 	lea    0x437c3(%rip),%rsi        # 44b2e1 <.Ldatab>
  407b1e:	ba 19 00 00 00       	mov    $0x19,%edx
  407b23:	48 8d 0d d0 37 04 00 	lea    0x437d0(%rip),%rcx        # 44b2fa <.Ldatac>
  407b2a:	41 b8 41 00 00 00    	mov    $0x41,%r8d
  407b30:	4c 8d 0d 04 38 04 00 	lea    0x43804(%rip),%r9        # 44b33b <.Ldatad>
  407b37:	41 ba 33 00 00 00    	mov    $0x33,%r10d
  407b3d:	48 0f b6 f8          	movzbq %al,%rdi
  407b41:	4c 89 14 24          	mov    %r10,(%rsp)
  407b45:	48 8d 05 54 75 00 00 	lea    0x7554(%rip),%rax        # 40f0a0 <pt_raise_msg>
  407b4c:	ff d0                	call   *%rax
  407b4e:	48 8d 05 eb 3c 00 00 	lea    0x3ceb(%rip),%rax        # 40b840 <pt_str_dec>
  407b55:	4c 89 ff             	mov    %r15,%rdi
  407b58:	ff d0                	call   *%rax
  407b5a:	48 8d 05 df 3c 00 00 	lea    0x3cdf(%rip),%rax        # 40b840 <pt_str_dec>
  407b61:	4c 89 f7             	mov    %r14,%rdi
  407b64:	ff d0                	call   *%rax
  407b66:	48 33 ff             	xor    %rdi,%rdi
  407b69:	48 8d 05 d0 3c 00 00 	lea    0x3cd0(%rip),%rax        # 40b840 <pt_str_dec>
  407b70:	ff d0                	call   *%rax
  407b72:	48 8d 05 17 76 00 00 	lea    0x7617(%rip),%rax        # 40f190 <pt_uncaught>
  407b79:	ff d0                	call   *%rax
  407b7b:	0f 0b                	ud2
  407b7d:	48 8d 3d 3b 35 04 00 	lea    0x4353b(%rip),%rdi        # 44b0bf <.Ldata6>
  407b84:	be 90 00 00 00       	mov    $0x90,%esi
  407b89:	48 8d 05 20 6c 00 00 	lea    0x6c20(%rip),%rax        # 40e7b0 <pt_fail>
  407b90:	ff d0                	call   *%rax
  407b92:	0f 0b                	ud2
  407b94:	00 00                	add    %al,(%rax)
  407b96:	00 00                	add    %al,(%rax)
  407b98:	e9 a8 c0 17 57       	jmp    57583c45 <_end+0x5712315d>
  407b9d:	3f                   	(bad)
  407b9e:	e8 a8 61 00 00       	call   40dd4b <_RINvNtCs40k4W9msRzi_5alloc3str17join_generic_copyehReECsk1kE6r7WBHz_11pythrust_rt+0x46b>
  407ba3:	00 00                	add    %al,(%rax)
  407ba5:	00 00                	add    %al,(%rax)
  407ba7:	00 0b                	add    %cl,(%rbx)
  407ba9:	d7                   	xlat   %ds:(%rbx)
  407baa:	a3 70 3d 0a d7 a3 32 	movabs %eax,0x32a3d70a3d70
  407bb1:	00 00 
  407bb3:	00 00                	add    %al,(%rax)
  407bb5:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:

/workspace/card817/probe-scoped/profile:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .text:

000000000040b700 <pt_str_concat>:
  40b700:	55                   	push   %rbp
  40b701:	41 57                	push   %r15
  40b703:	41 56                	push   %r14
  40b705:	41 55                	push   %r13
  40b707:	41 54                	push   %r12
  40b709:	53                   	push   %rbx
  40b70a:	48 83 ec 38          	sub    $0x38,%rsp
  40b70e:	48 89 f3             	mov    %rsi,%rbx
  40b711:	49 89 fe             	mov    %rdi,%r14
  40b714:	0f 10 07             	movups (%rdi),%xmm0
  40b717:	0f 10 0e             	movups (%rsi),%xmm1
  40b71a:	0f 11 44 24 18       	movups %xmm0,0x18(%rsp)
  40b71f:	0f 11 4c 24 28       	movups %xmm1,0x28(%rsp)
  40b724:	48 89 e7             	mov    %rsp,%rdi
  40b727:	48 8d 74 24 18       	lea    0x18(%rsp),%rsi
  40b72c:	ba 02 00 00 00       	mov    $0x2,%edx
  40b731:	b9 01 00 00 00       	mov    $0x1,%ecx
  40b736:	45 31 c0             	xor    %r8d,%r8d
  40b739:	e8 a2 21 00 00       	call   40d8e0 <_RINvNtCs40k4W9msRzi_5alloc3str17join_generic_copyehReECsk1kE6r7WBHz_11pythrust_rt>
  40b73e:	49 8b 6e 18          	mov    0x18(%r14),%rbp
  40b742:	4c 8b 6b 18          	mov    0x18(%rbx),%r13
  40b746:	4c 8b 3c 24          	mov    (%rsp),%r15
  40b74a:	48 8b 5c 24 10       	mov    0x10(%rsp),%rbx
  40b74f:	49 39 df             	cmp    %rbx,%r15
  40b752:	76 36                	jbe    40b78a <pt_str_concat+0x8a>
  40b754:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
  40b759:	48 85 db             	test   %rbx,%rbx
  40b75c:	74 33                	je     40b791 <pt_str_concat+0x91>
  40b75e:	ba 01 00 00 00       	mov    $0x1,%edx
  40b763:	4c 89 e7             	mov    %r12,%rdi
  40b766:	4c 89 fe             	mov    %r15,%rsi
  40b769:	48 89 d9             	mov    %rbx,%rcx
  40b76c:	ff 15 86 45 05 00    	call   *0x54586(%rip)        # 45fcf8 <_DYNAMIC+0x770>
  40b772:	49 89 c6             	mov    %rax,%r14
  40b775:	48 85 c0             	test   %rax,%rax
  40b778:	75 2e                	jne    40b7a8 <pt_str_concat+0xa8>
  40b77a:	bf 01 00 00 00       	mov    $0x1,%edi
  40b77f:	48 89 de             	mov    %rbx,%rsi
  40b782:	ff 15 50 44 05 00    	call   *0x54450(%rip)        # 45fbd8 <_DYNAMIC+0x650>
  40b788:	eb 6e                	jmp    40b7f8 <pt_str_concat+0xf8>
  40b78a:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
  40b78f:	eb 17                	jmp    40b7a8 <pt_str_concat+0xa8>
  40b791:	41 be 01 00 00 00    	mov    $0x1,%r14d
  40b797:	ba 01 00 00 00       	mov    $0x1,%edx
  40b79c:	4c 89 e7             	mov    %r12,%rdi
  40b79f:	4c 89 fe             	mov    %r15,%rsi
  40b7a2:	ff 15 80 47 05 00    	call   *0x54780(%rip)        # 45ff28 <_DYNAMIC+0x9a0>
  40b7a8:	ff 15 62 42 05 00    	call   *0x54262(%rip)        # 45fa10 <_DYNAMIC+0x488>
  40b7ae:	bf 20 00 00 00       	mov    $0x20,%edi
  40b7b3:	be 08 00 00 00       	mov    $0x8,%esi
  40b7b8:	ff 15 92 42 05 00    	call   *0x54292(%rip)        # 45fa50 <_DYNAMIC+0x4c8>
  40b7be:	48 85 c0             	test   %rax,%rax
  40b7c1:	74 25                	je     40b7e8 <pt_str_concat+0xe8>
  40b7c3:	49 01 ed             	add    %rbp,%r13
  40b7c6:	4c 89 30             	mov    %r14,(%rax)
  40b7c9:	48 89 58 08          	mov    %rbx,0x8(%rax)
  40b7cd:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
  40b7d4:	00 
  40b7d5:	4c 89 68 18          	mov    %r13,0x18(%rax)
  40b7d9:	48 83 c4 38          	add    $0x38,%rsp
  40b7dd:	5b                   	pop    %rbx
  40b7de:	41 5c                	pop    %r12
  40b7e0:	41 5d                	pop    %r13
  40b7e2:	41 5e                	pop    %r14
  40b7e4:	41 5f                	pop    %r15
  40b7e6:	5d                   	pop    %rbp
  40b7e7:	c3                   	ret
  40b7e8:	bf 08 00 00 00       	mov    $0x8,%edi
  40b7ed:	be 20 00 00 00       	mov    $0x20,%esi
  40b7f2:	ff 15 58 44 05 00    	call   *0x54458(%rip)        # 45fc50 <_DYNAMIC+0x6c8>
  40b7f8:	0f 0b                	ud2
  40b7fa:	ba 01 00 00 00       	mov    $0x1,%edx
  40b7ff:	4c 89 e7             	mov    %r12,%rdi
  40b802:	4c 89 fe             	mov    %r15,%rsi
  40b805:	ff 15 1d 47 05 00    	call   *0x5471d(%rip)        # 45ff28 <_DYNAMIC+0x9a0>
  40b80b:	ff 15 8f 40 05 00    	call   *0x5408f(%rip)        # 45f8a0 <_DYNAMIC+0x318>
  40b811:	ff 15 89 40 05 00    	call   *0x54089(%rip)        # 45f8a0 <_DYNAMIC+0x318>
  40b817:	48 85 db             	test   %rbx,%rbx
  40b81a:	75 06                	jne    40b822 <pt_str_concat+0x122>
  40b81c:	ff 15 7e 40 05 00    	call   *0x5407e(%rip)        # 45f8a0 <_DYNAMIC+0x318>
  40b822:	ba 01 00 00 00       	mov    $0x1,%edx
  40b827:	4c 89 f7             	mov    %r14,%rdi
  40b82a:	48 89 de             	mov    %rbx,%rsi
  40b82d:	ff 15 f5 46 05 00    	call   *0x546f5(%rip)        # 45ff28 <_DYNAMIC+0x9a0>
  40b833:	ff 15 67 40 05 00    	call   *0x54067(%rip)        # 45f8a0 <_DYNAMIC+0x318>

Disassembly of section .fini:

/workspace/card817/probe-scoped/profile:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .text:

000000000040b8c0 <pt_str_from_int>:
  40b8c0:	41 57                	push   %r15
  40b8c2:	41 56                	push   %r14
  40b8c4:	41 54                	push   %r12
  40b8c6:	53                   	push   %rbx
  40b8c7:	48 83 ec 38          	sub    $0x38,%rsp
  40b8cb:	0f 28 05 0e 30 04 00 	movaps 0x4300e(%rip),%xmm0        # 44e8e0 <anon.13df560d6b83da2c10641ad9b2156d4a.31.llvm.366253514071984098+0x51>
  40b8d2:	0f 29 04 24          	movaps %xmm0,(%rsp)
  40b8d6:	c7 44 24 0f 30 30 30 	movl   $0x30303030,0xf(%rsp)
  40b8dd:	30 
  40b8de:	48 89 f9             	mov    %rdi,%rcx
  40b8e1:	48 f7 d9             	neg    %rcx
  40b8e4:	48 0f 48 cf          	cmovs  %rdi,%rcx
  40b8e8:	48 ba cd cc cc cc cc 	movabs $0xcccccccccccccccd,%rdx
  40b8ef:	cc cc cc 
  40b8f2:	48 89 c8             	mov    %rcx,%rax
  40b8f5:	48 f7 e2             	mul    %rdx
  40b8f8:	48 c1 ea 03          	shr    $0x3,%rdx
  40b8fc:	8d 04 12             	lea    (%rdx,%rdx,1),%eax
  40b8ff:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40b902:	89 ce                	mov    %ecx,%esi
  40b904:	29 c6                	sub    %eax,%esi
  40b906:	40 80 ce 30          	or     $0x30,%sil
  40b90a:	40 88 74 24 13       	mov    %sil,0x13(%rsp)
  40b90f:	48 83 f9 0a          	cmp    $0xa,%rcx
  40b913:	73 0f                	jae    40b924 <pt_str_from_int+0x64>
  40b915:	b8 13 00 00 00       	mov    $0x13,%eax
  40b91a:	b9 12 00 00 00       	mov    $0x12,%ecx
  40b91f:	e9 f1 04 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40b924:	49 b8 9a 99 99 99 99 	movabs $0x199999999999999a,%r8
  40b92b:	99 99 19 
  40b92e:	48 89 d0             	mov    %rdx,%rax
  40b931:	48 89 d6             	mov    %rdx,%rsi
  40b934:	49 f7 e0             	mul    %r8
  40b937:	01 d2                	add    %edx,%edx
  40b939:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40b93c:	29 c6                	sub    %eax,%esi
  40b93e:	40 80 ce 30          	or     $0x30,%sil
  40b942:	40 88 74 24 12       	mov    %sil,0x12(%rsp)
  40b947:	48 83 f9 64          	cmp    $0x64,%rcx
  40b94b:	73 0f                	jae    40b95c <pt_str_from_int+0x9c>
  40b94d:	b8 12 00 00 00       	mov    $0x12,%eax
  40b952:	b9 11 00 00 00       	mov    $0x11,%ecx
  40b957:	e9 b9 04 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40b95c:	48 89 c8             	mov    %rcx,%rax
  40b95f:	48 c1 e8 02          	shr    $0x2,%rax
  40b963:	48 ba c3 f5 28 5c 8f 	movabs $0x28f5c28f5c28f5c3,%rdx
  40b96a:	c2 f5 28 
  40b96d:	48 f7 e2             	mul    %rdx
  40b970:	48 89 d6             	mov    %rdx,%rsi
  40b973:	48 c1 ee 02          	shr    $0x2,%rsi
  40b977:	48 89 f0             	mov    %rsi,%rax
  40b97a:	49 f7 e0             	mul    %r8
  40b97d:	01 d2                	add    %edx,%edx
  40b97f:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40b982:	29 c6                	sub    %eax,%esi
  40b984:	40 80 ce 30          	or     $0x30,%sil
  40b988:	40 88 74 24 11       	mov    %sil,0x11(%rsp)
  40b98d:	48 81 f9 e8 03 00 00 	cmp    $0x3e8,%rcx
  40b994:	73 0f                	jae    40b9a5 <pt_str_from_int+0xe5>
  40b996:	b8 11 00 00 00       	mov    $0x11,%eax
  40b99b:	b9 10 00 00 00       	mov    $0x10,%ecx
  40b9a0:	e9 70 04 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40b9a5:	48 89 c8             	mov    %rcx,%rax
  40b9a8:	48 c1 e8 03          	shr    $0x3,%rax
  40b9ac:	48 ba cf f7 53 e3 a5 	movabs $0x20c49ba5e353f7cf,%rdx
  40b9b3:	9b c4 20 
  40b9b6:	48 f7 e2             	mul    %rdx
  40b9b9:	48 89 d6             	mov    %rdx,%rsi
  40b9bc:	48 c1 ee 04          	shr    $0x4,%rsi
  40b9c0:	48 89 f0             	mov    %rsi,%rax
  40b9c3:	49 f7 e0             	mul    %r8
  40b9c6:	01 d2                	add    %edx,%edx
  40b9c8:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40b9cb:	29 c6                	sub    %eax,%esi
  40b9cd:	40 80 ce 30          	or     $0x30,%sil
  40b9d1:	40 88 74 24 10       	mov    %sil,0x10(%rsp)
  40b9d6:	48 81 f9 10 27 00 00 	cmp    $0x2710,%rcx
  40b9dd:	73 0f                	jae    40b9ee <pt_str_from_int+0x12e>
  40b9df:	b8 10 00 00 00       	mov    $0x10,%eax
  40b9e4:	b9 0f 00 00 00       	mov    $0xf,%ecx
  40b9e9:	e9 27 04 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40b9ee:	48 ba 4b 59 86 38 d6 	movabs $0x346dc5d63886594b,%rdx
  40b9f5:	c5 6d 34 
  40b9f8:	48 89 c8             	mov    %rcx,%rax
  40b9fb:	48 f7 e2             	mul    %rdx
  40b9fe:	48 89 d6             	mov    %rdx,%rsi
  40ba01:	48 c1 ee 0b          	shr    $0xb,%rsi
  40ba05:	48 89 f0             	mov    %rsi,%rax
  40ba08:	49 f7 e0             	mul    %r8
  40ba0b:	01 d2                	add    %edx,%edx
  40ba0d:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40ba10:	29 c6                	sub    %eax,%esi
  40ba12:	40 80 ce 30          	or     $0x30,%sil
  40ba16:	40 88 74 24 0f       	mov    %sil,0xf(%rsp)
  40ba1b:	48 81 f9 a0 86 01 00 	cmp    $0x186a0,%rcx
  40ba22:	73 0f                	jae    40ba33 <pt_str_from_int+0x173>
  40ba24:	b8 0f 00 00 00       	mov    $0xf,%eax
  40ba29:	b9 0e 00 00 00       	mov    $0xe,%ecx
  40ba2e:	e9 e2 03 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40ba33:	48 89 c8             	mov    %rcx,%rax
  40ba36:	48 c1 e8 05          	shr    $0x5,%rax
  40ba3a:	48 ba 43 78 b4 71 c4 	movabs $0xa7c5ac471b47843,%rdx
  40ba41:	5a 7c 0a 
  40ba44:	48 f7 e2             	mul    %rdx
  40ba47:	48 89 d6             	mov    %rdx,%rsi
  40ba4a:	48 c1 ee 07          	shr    $0x7,%rsi
  40ba4e:	48 89 f0             	mov    %rsi,%rax
  40ba51:	49 f7 e0             	mul    %r8
  40ba54:	01 d2                	add    %edx,%edx
  40ba56:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40ba59:	29 c6                	sub    %eax,%esi
  40ba5b:	40 80 ce 30          	or     $0x30,%sil
  40ba5f:	40 88 74 24 0e       	mov    %sil,0xe(%rsp)
  40ba64:	48 81 f9 40 42 0f 00 	cmp    $0xf4240,%rcx
  40ba6b:	73 0f                	jae    40ba7c <pt_str_from_int+0x1bc>
  40ba6d:	b8 0e 00 00 00       	mov    $0xe,%eax
  40ba72:	b9 0d 00 00 00       	mov    $0xd,%ecx
  40ba77:	e9 99 03 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40ba7c:	48 ba db 34 b6 d7 82 	movabs $0x431bde82d7b634db,%rdx
  40ba83:	de 1b 43 
  40ba86:	48 89 c8             	mov    %rcx,%rax
  40ba89:	48 f7 e2             	mul    %rdx
  40ba8c:	48 89 d6             	mov    %rdx,%rsi
  40ba8f:	48 c1 ee 12          	shr    $0x12,%rsi
  40ba93:	48 89 f0             	mov    %rsi,%rax
  40ba96:	49 f7 e0             	mul    %r8
  40ba99:	01 d2                	add    %edx,%edx
  40ba9b:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40ba9e:	29 c6                	sub    %eax,%esi
  40baa0:	40 80 ce 30          	or     $0x30,%sil
  40baa4:	40 88 74 24 0d       	mov    %sil,0xd(%rsp)
  40baa9:	48 81 f9 80 96 98 00 	cmp    $0x989680,%rcx
  40bab0:	73 0f                	jae    40bac1 <pt_str_from_int+0x201>
  40bab2:	b8 0d 00 00 00       	mov    $0xd,%eax
  40bab7:	b9 0c 00 00 00       	mov    $0xc,%ecx
  40babc:	e9 54 03 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40bac1:	48 ba bd 42 7a e5 d5 	movabs $0xd6bf94d5e57a42bd,%rdx
  40bac8:	94 bf d6 
  40bacb:	48 89 c8             	mov    %rcx,%rax
  40bace:	48 f7 e2             	mul    %rdx
  40bad1:	48 89 d6             	mov    %rdx,%rsi
  40bad4:	48 c1 ee 17          	shr    $0x17,%rsi
  40bad8:	48 89 f0             	mov    %rsi,%rax
  40badb:	49 f7 e0             	mul    %r8
  40bade:	01 d2                	add    %edx,%edx
  40bae0:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40bae3:	29 c6                	sub    %eax,%esi
  40bae5:	40 80 ce 30          	or     $0x30,%sil
  40bae9:	40 88 74 24 0c       	mov    %sil,0xc(%rsp)
  40baee:	48 81 f9 00 e1 f5 05 	cmp    $0x5f5e100,%rcx
  40baf5:	73 0f                	jae    40bb06 <pt_str_from_int+0x246>
  40baf7:	b8 0c 00 00 00       	mov    $0xc,%eax
  40bafc:	b9 0b 00 00 00       	mov    $0xb,%ecx
  40bb01:	e9 0f 03 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40bb06:	48 ba fd ce 61 84 11 	movabs $0xabcc77118461cefd,%rdx
  40bb0d:	77 cc ab 
  40bb10:	48 89 c8             	mov    %rcx,%rax
  40bb13:	48 f7 e2             	mul    %rdx
  40bb16:	48 89 d6             	mov    %rdx,%rsi
  40bb19:	48 c1 ee 1a          	shr    $0x1a,%rsi
  40bb1d:	48 89 f0             	mov    %rsi,%rax
  40bb20:	49 f7 e0             	mul    %r8
  40bb23:	01 d2                	add    %edx,%edx
  40bb25:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40bb28:	29 c6                	sub    %eax,%esi
  40bb2a:	40 80 ce 30          	or     $0x30,%sil
  40bb2e:	40 88 74 24 0b       	mov    %sil,0xb(%rsp)
  40bb33:	48 81 f9 00 ca 9a 3b 	cmp    $0x3b9aca00,%rcx
  40bb3a:	73 0f                	jae    40bb4b <pt_str_from_int+0x28b>
  40bb3c:	b8 0b 00 00 00       	mov    $0xb,%eax
  40bb41:	b9 0a 00 00 00       	mov    $0xa,%ecx
  40bb46:	e9 ca 02 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40bb4b:	48 89 c8             	mov    %rcx,%rax
  40bb4e:	48 c1 e8 09          	shr    $0x9,%rax
  40bb52:	48 ba 53 5a 9b a0 2f 	movabs $0x44b82fa09b5a53,%rdx
  40bb59:	b8 44 00 
  40bb5c:	48 f7 e2             	mul    %rdx
  40bb5f:	48 89 d6             	mov    %rdx,%rsi
  40bb62:	48 c1 ee 0b          	shr    $0xb,%rsi
  40bb66:	48 89 f0             	mov    %rsi,%rax
  40bb69:	49 f7 e0             	mul    %r8
  40bb6c:	01 d2                	add    %edx,%edx
  40bb6e:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
  40bb71:	29 c6                	sub    %eax,%esi
  40bb73:	40 80 ce 30          	or     $0x30,%sil
  40bb77:	40 88 74 24 0a       	mov    %sil,0xa(%rsp)
  40bb7c:	48 89 c8             	mov    %rcx,%rax
  40bb7f:	48 c1 e8 0a          	shr    $0xa,%rax
  40bb83:	48 3d f9 02 95 00    	cmp    $0x9502f9,%rax
  40bb89:	73 0f                	jae    40bb9a <pt_str_from_int+0x2da>
  40bb8b:	b8 0a 00 00 00       	mov    $0xa,%eax
  40bb90:	b9 09 00 00 00       	mov    $0x9,%ecx
  40bb95:	e9 7b 02 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40bb9a:	48 ba bf d5 ed bd ce 	movabs $0xdbe6fecebdedd5bf,%rdx
  40bba1:	fe e6 db 
  40bba4:	48 89 c8             	mov    %rcx,%rax
  40bba7:	48 f7 e2             	mul    %rdx
  40bbaa:	48 c1 ea 21          	shr    $0x21,%rdx
  40bbae:	48 69 c2 67 66 66 66 	imul   $0x66666667,%rdx,%rax
  40bbb5:	48 c1 e8 22          	shr    $0x22,%rax
  40bbb9:	01 c0                	add    %eax,%eax
  40bbbb:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bbbe:	29 c2                	sub    %eax,%edx
  40bbc0:	80 ca 30             	or     $0x30,%dl
  40bbc3:	88 54 24 09          	mov    %dl,0x9(%rsp)
  40bbc7:	48 89 c8             	mov    %rcx,%rax
  40bbca:	48 c1 e8 0b          	shr    $0xb,%rax
  40bbce:	48 3d dd 0e e9 02    	cmp    $0x2e90edd,%rax
  40bbd4:	73 0f                	jae    40bbe5 <pt_str_from_int+0x325>
  40bbd6:	b8 09 00 00 00       	mov    $0x9,%eax
  40bbdb:	b9 08 00 00 00       	mov    $0x8,%ecx
  40bbe0:	e9 30 02 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40bbe5:	48 ba ff aa 24 cb 0b 	movabs $0xafebff0bcb24aaff,%rdx
  40bbec:	ff eb af 
  40bbef:	48 89 c8             	mov    %rcx,%rax
  40bbf2:	48 f7 e2             	mul    %rdx
  40bbf5:	48 c1 ea 24          	shr    $0x24,%rdx
  40bbf9:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
  40bc00:	48 c1 e8 20          	shr    $0x20,%rax
  40bc04:	01 c0                	add    %eax,%eax
  40bc06:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bc09:	29 c2                	sub    %eax,%edx
  40bc0b:	80 ca 30             	or     $0x30,%dl
  40bc0e:	88 54 24 08          	mov    %dl,0x8(%rsp)
  40bc12:	48 89 c8             	mov    %rcx,%rax
  40bc15:	48 c1 e8 0c          	shr    $0xc,%rax
  40bc19:	48 3d 51 4a 8d 0e    	cmp    $0xe8d4a51,%rax
  40bc1f:	73 0f                	jae    40bc30 <pt_str_from_int+0x370>
  40bc21:	b8 08 00 00 00       	mov    $0x8,%eax
  40bc26:	b9 07 00 00 00       	mov    $0x7,%ecx
  40bc2b:	e9 e5 01 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40bc30:	48 ba 33 22 d4 5b 02 	movabs $0x232f33025bd42233,%rdx
  40bc37:	33 2f 23 
  40bc3a:	48 89 c8             	mov    %rcx,%rax
  40bc3d:	48 f7 e2             	mul    %rdx
  40bc40:	48 c1 ea 25          	shr    $0x25,%rdx
  40bc44:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
  40bc4b:	48 c1 e8 20          	shr    $0x20,%rax
  40bc4f:	01 c0                	add    %eax,%eax
  40bc51:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bc54:	29 c2                	sub    %eax,%edx
  40bc56:	80 ca 30             	or     $0x30,%dl
  40bc59:	88 54 24 07          	mov    %dl,0x7(%rsp)
  40bc5d:	48 89 c8             	mov    %rcx,%rax
  40bc60:	48 c1 e8 0d          	shr    $0xd,%rax
  40bc64:	48 3d 95 73 c2 48    	cmp    $0x48c27395,%rax
  40bc6a:	73 0f                	jae    40bc7b <pt_str_from_int+0x3bb>
  40bc6c:	b8 07 00 00 00       	mov    $0x7,%eax
  40bc71:	b9 06 00 00 00       	mov    $0x6,%ecx
  40bc76:	e9 9a 01 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40bc7b:	48 ba 85 03 ed 92 d0 	movabs $0x384b84d092ed0385,%rdx
  40bc82:	84 4b 38 
  40bc85:	48 89 c8             	mov    %rcx,%rax
  40bc88:	48 f7 e2             	mul    %rdx
  40bc8b:	48 c1 ea 29          	shr    $0x29,%rdx
  40bc8f:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
  40bc96:	48 c1 e8 20          	shr    $0x20,%rax
  40bc9a:	01 c0                	add    %eax,%eax
  40bc9c:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bc9f:	29 c2                	sub    %eax,%edx
  40bca1:	80 ca 30             	or     $0x30,%dl
  40bca4:	88 54 24 06          	mov    %dl,0x6(%rsp)
  40bca8:	48 b8 00 40 7a 10 f3 	movabs $0x5af3107a4000,%rax
  40bcaf:	5a 00 00 
  40bcb2:	48 39 c1             	cmp    %rax,%rcx
  40bcb5:	73 0f                	jae    40bcc6 <pt_str_from_int+0x406>
  40bcb7:	b8 06 00 00 00       	mov    $0x6,%eax
  40bcbc:	b9 05 00 00 00       	mov    $0x5,%ecx
  40bcc1:	e9 4f 01 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40bcc6:	48 ba 81 cd 95 50 c3 	movabs $0xb424dc35095cd81,%rdx
  40bccd:	4d 42 0b 
  40bcd0:	48 89 c8             	mov    %rcx,%rax
  40bcd3:	48 f7 e2             	mul    %rdx
  40bcd6:	48 c1 ea 2a          	shr    $0x2a,%rdx
  40bcda:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
  40bce1:	48 c1 e8 20          	shr    $0x20,%rax
  40bce5:	01 c0                	add    %eax,%eax
  40bce7:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bcea:	29 c2                	sub    %eax,%edx
  40bcec:	80 ca 30             	or     $0x30,%dl
  40bcef:	88 54 24 05          	mov    %dl,0x5(%rsp)
  40bcf3:	48 b8 00 80 c6 a4 7e 	movabs $0x38d7ea4c68000,%rax
  40bcfa:	8d 03 00 
  40bcfd:	48 39 c1             	cmp    %rax,%rcx
  40bd00:	73 0f                	jae    40bd11 <pt_str_from_int+0x451>
  40bd02:	b8 05 00 00 00       	mov    $0x5,%eax
  40bd07:	b9 04 00 00 00       	mov    $0x4,%ecx
  40bd0c:	e9 04 01 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40bd11:	48 89 c8             	mov    %rcx,%rax
  40bd14:	48 c1 e8 0f          	shr    $0xf,%rax
  40bd18:	48 ba c3 ea dc f3 75 	movabs $0x24075f3dceac3,%rdx
  40bd1f:	40 02 00 
  40bd22:	48 f7 e2             	mul    %rdx
  40bd25:	48 c1 ea 14          	shr    $0x14,%rdx
  40bd29:	69 c2 67 66 00 00    	imul   $0x6667,%edx,%eax
  40bd2f:	c1 e8 11             	shr    $0x11,%eax
  40bd32:	83 e0 fe             	and    $0xfffffffe,%eax
  40bd35:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bd38:	29 c2                	sub    %eax,%edx
  40bd3a:	80 ca 30             	or     $0x30,%dl
  40bd3d:	88 54 24 04          	mov    %dl,0x4(%rsp)
  40bd41:	48 b8 00 00 c1 6f f2 	movabs $0x2386f26fc10000,%rax
  40bd48:	86 23 00 
  40bd4b:	48 39 c1             	cmp    %rax,%rcx
  40bd4e:	73 0f                	jae    40bd5f <pt_str_from_int+0x49f>
  40bd50:	b8 04 00 00 00       	mov    $0x4,%eax
  40bd55:	b9 03 00 00 00       	mov    $0x3,%ecx
  40bd5a:	e9 b6 00 00 00       	jmp    40be15 <pt_str_from_int+0x555>
  40bd5f:	48 ba 57 78 13 b1 2f 	movabs $0x39a5652fb1137857,%rdx
  40bd66:	65 a5 39 
  40bd69:	48 89 c8             	mov    %rcx,%rax
  40bd6c:	48 f7 e2             	mul    %rdx
  40bd6f:	48 c1 ea 33          	shr    $0x33,%rdx
  40bd73:	69 c2 9a 19 00 00    	imul   $0x199a,%edx,%eax
  40bd79:	c1 e8 0f             	shr    $0xf,%eax
  40bd7c:	83 e0 fe             	and    $0xfffffffe,%eax
  40bd7f:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bd82:	29 c2                	sub    %eax,%edx
  40bd84:	80 ca 30             	or     $0x30,%dl
  40bd87:	88 54 24 03          	mov    %dl,0x3(%rsp)
  40bd8b:	48 b8 00 00 8a 5d 78 	movabs $0x16345785d8a0000,%rax
  40bd92:	45 63 01 
  40bd95:	48 39 c1             	cmp    %rax,%rcx
  40bd98:	73 0c                	jae    40bda6 <pt_str_from_int+0x4e6>
  40bd9a:	b8 03 00 00 00       	mov    $0x3,%eax
  40bd9f:	b9 02 00 00 00       	mov    $0x2,%ecx
  40bda4:	eb 6f                	jmp    40be15 <pt_str_from_int+0x555>
  40bda6:	48 89 c8             	mov    %rcx,%rax
  40bda9:	48 c1 e8 11          	shr    $0x11,%rax
  40bdad:	48 ba 53 1b 19 d5 3b 	movabs $0x5c3bd5191b53,%rdx
  40bdb4:	5c 00 00 
  40bdb7:	48 f7 e2             	mul    %rdx
  40bdba:	c1 ea 16             	shr    $0x16,%edx
  40bdbd:	69 c2 cd 00 00 00    	imul   $0xcd,%edx,%eax
  40bdc3:	c1 e8 0a             	shr    $0xa,%eax
  40bdc6:	83 e0 fe             	and    $0xfffffffe,%eax
  40bdc9:	8d 04 80             	lea    (%rax,%rax,4),%eax
  40bdcc:	28 c2                	sub    %al,%dl
  40bdce:	80 ca 30             	or     $0x30,%dl
  40bdd1:	88 54 24 02          	mov    %dl,0x2(%rsp)
  40bdd5:	48 b8 00 00 64 a7 b3 	movabs $0xde0b6b3a7640000,%rax
  40bddc:	b6 e0 0d 
  40bddf:	48 39 c1             	cmp    %rax,%rcx
  40bde2:	73 0c                	jae    40bdf0 <pt_str_from_int+0x530>
  40bde4:	b8 02 00 00 00       	mov    $0x2,%eax
  40bde9:	b9 01 00 00 00       	mov    $0x1,%ecx
  40bdee:	eb 25                	jmp    40be15 <pt_str_from_int+0x555>
  40bdf0:	48 c1 e9 12          	shr    $0x12,%rcx
  40bdf4:	48 ba 0f 49 47 77 c9 	movabs $0x49c97747490f,%rdx
  40bdfb:	49 00 00 
  40bdfe:	48 89 c8             	mov    %rcx,%rax
  40be01:	48 f7 e2             	mul    %rdx
  40be04:	c1 ea 18             	shr    $0x18,%edx
  40be07:	80 ca 30             	or     $0x30,%dl
  40be0a:	88 54 24 01          	mov    %dl,0x1(%rsp)
  40be0e:	b8 01 00 00 00       	mov    $0x1,%eax
  40be13:	31 c9                	xor    %ecx,%ecx
  40be15:	48 85 ff             	test   %rdi,%rdi
  40be18:	79 07                	jns    40be21 <pt_str_from_int+0x561>
  40be1a:	c6 04 0c 2d          	movb   $0x2d,(%rsp,%rcx,1)
  40be1e:	48 89 c8             	mov    %rcx,%rax
  40be21:	ba 14 00 00 00       	mov    $0x14,%edx
  40be26:	48 29 c2             	sub    %rax,%rdx
  40be29:	48 8d 34 04          	lea    (%rsp,%rax,1),%rsi
  40be2d:	48 8d 7c 24 20       	lea    0x20(%rsp),%rdi
  40be32:	ff 15 b0 3b 05 00    	call   *0x53bb0(%rip)        # 45f9e8 <_DYNAMIC+0x460>
  40be38:	80 7c 24 20 00       	cmpb   $0x0,0x20(%rsp)
  40be3d:	0f 85 98 00 00 00    	jne    40bedb <pt_str_from_int+0x61b>
  40be43:	48 8b 5c 24 30       	mov    0x30(%rsp),%rbx
  40be48:	48 85 db             	test   %rbx,%rbx
  40be4b:	79 14                	jns    40be61 <pt_str_from_int+0x5a1>
  40be4d:	45 31 e4             	xor    %r12d,%r12d
  40be50:	4c 89 e7             	mov    %r12,%rdi
  40be53:	48 89 de             	mov    %rbx,%rsi
  40be56:	ff 15 7c 3d 05 00    	call   *0x53d7c(%rip)        # 45fbd8 <_DYNAMIC+0x650>
  40be5c:	e9 a5 00 00 00       	jmp    40bf06 <pt_str_from_int+0x646>
  40be61:	74 38                	je     40be9b <pt_str_from_int+0x5db>
  40be63:	4c 8b 7c 24 28       	mov    0x28(%rsp),%r15
  40be68:	ff 15 a2 3b 05 00    	call   *0x53ba2(%rip)        # 45fa10 <_DYNAMIC+0x488>
  40be6e:	41 bc 01 00 00 00    	mov    $0x1,%r12d
  40be74:	be 01 00 00 00       	mov    $0x1,%esi
  40be79:	48 89 df             	mov    %rbx,%rdi
  40be7c:	ff 15 ce 3b 05 00    	call   *0x53bce(%rip)        # 45fa50 <_DYNAMIC+0x4c8>
  40be82:	48 85 c0             	test   %rax,%rax
  40be85:	74 c9                	je     40be50 <pt_str_from_int+0x590>
  40be87:	49 89 c6             	mov    %rax,%r14
  40be8a:	48 89 c7             	mov    %rax,%rdi
  40be8d:	4c 89 fe             	mov    %r15,%rsi
  40be90:	48 89 da             	mov    %rbx,%rdx
  40be93:	ff 15 ef 40 05 00    	call   *0x540ef(%rip)        # 45ff88 <memcpy@GLIBC_2.14>
  40be99:	eb 06                	jmp    40bea1 <pt_str_from_int+0x5e1>
  40be9b:	41 be 01 00 00 00    	mov    $0x1,%r14d
  40bea1:	ff 15 69 3b 05 00    	call   *0x53b69(%rip)        # 45fa10 <_DYNAMIC+0x488>
  40bea7:	bf 20 00 00 00       	mov    $0x20,%edi
  40beac:	be 08 00 00 00       	mov    $0x8,%esi
  40beb1:	ff 15 99 3b 05 00    	call   *0x53b99(%rip)        # 45fa50 <_DYNAMIC+0x4c8>
  40beb7:	48 85 c0             	test   %rax,%rax
  40beba:	74 3a                	je     40bef6 <pt_str_from_int+0x636>
  40bebc:	4c 89 30             	mov    %r14,(%rax)
  40bebf:	48 89 58 08          	mov    %rbx,0x8(%rax)
  40bec3:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
  40beca:	00 
  40becb:	48 89 58 18          	mov    %rbx,0x18(%rax)
  40becf:	48 83 c4 38          	add    $0x38,%rsp
  40bed3:	5b                   	pop    %rbx
  40bed4:	41 5c                	pop    %r12
  40bed6:	41 5e                	pop    %r14
  40bed8:	41 5f                	pop    %r15
  40beda:	c3                   	ret
  40bedb:	48 8d 3d 0e 2e 04 00 	lea    0x42e0e(%rip),%rdi        # 44ecf0 <anon.13df560d6b83da2c10641ad9b2156d4a.31.llvm.366253514071984098+0x461>
  40bee2:	48 8d 15 17 15 05 00 	lea    0x51517(%rip),%rdx        # 45d400 <__do_global_dtors_aux_fini_array_entry+0x110>
  40bee9:	be 97 00 00 00       	mov    $0x97,%esi
  40beee:	ff 15 0c 3c 05 00    	call   *0x53c0c(%rip)        # 45fb00 <_DYNAMIC+0x578>
  40bef4:	eb 10                	jmp    40bf06 <pt_str_from_int+0x646>
  40bef6:	bf 08 00 00 00       	mov    $0x8,%edi
  40befb:	be 20 00 00 00       	mov    $0x20,%esi
  40bf00:	ff 15 4a 3d 05 00    	call   *0x53d4a(%rip)        # 45fc50 <_DYNAMIC+0x6c8>
  40bf06:	0f 0b                	ud2
  40bf08:	48 85 db             	test   %rbx,%rbx
  40bf0b:	74 17                	je     40bf24 <pt_str_from_int+0x664>
  40bf0d:	ba 01 00 00 00       	mov    $0x1,%edx
  40bf12:	4c 89 f7             	mov    %r14,%rdi
  40bf15:	48 89 de             	mov    %rbx,%rsi
  40bf18:	ff 15 0a 40 05 00    	call   *0x5400a(%rip)        # 45ff28 <_DYNAMIC+0x9a0>
  40bf1e:	ff 15 7c 39 05 00    	call   *0x5397c(%rip)        # 45f8a0 <_DYNAMIC+0x318>
  40bf24:	ff 15 76 39 05 00    	call   *0x53976(%rip)        # 45f8a0 <_DYNAMIC+0x318>

Disassembly of section .fini:

/workspace/card817/probe-scoped/profile:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .text:

000000000040c1c0 <pt_str_new>:
  40c1c0:	41 57                	push   %r15
  40c1c2:	41 56                	push   %r14
  40c1c4:	53                   	push   %rbx
  40c1c5:	48 83 ec 20          	sub    $0x20,%rsp
  40c1c9:	48 89 f2             	mov    %rsi,%rdx
  40c1cc:	48 89 fe             	mov    %rdi,%rsi
  40c1cf:	48 8d 7c 24 08       	lea    0x8(%rsp),%rdi
  40c1d4:	ff 15 f6 3c 05 00    	call   *0x53cf6(%rip)        # 45fed0 <_DYNAMIC+0x948>
  40c1da:	48 83 7c 24 08 ff    	cmpq   $0xffffffffffffffff,0x8(%rsp)
  40c1e0:	74 25                	je     40c207 <pt_str_new+0x47>
  40c1e2:	48 8d 7c 24 08       	lea    0x8(%rsp),%rdi
  40c1e7:	ff 15 03 3c 05 00    	call   *0x53c03(%rip)        # 45fdf0 <_DYNAMIC+0x868>
  40c1ed:	49 89 c6             	mov    %rax,%r14
  40c1f0:	48 89 d3             	mov    %rdx,%rbx
  40c1f3:	48 83 fb 20          	cmp    $0x20,%rbx
  40c1f7:	73 64                	jae    40c25d <pt_str_new+0x9d>
  40c1f9:	4c 89 f7             	mov    %r14,%rdi
  40c1fc:	48 89 de             	mov    %rbx,%rsi
  40c1ff:	ff 15 fb 3a 05 00    	call   *0x53afb(%rip)        # 45fd00 <_DYNAMIC+0x778>
  40c205:	eb 62                	jmp    40c269 <pt_str_new+0xa9>
  40c207:	4c 8b 7c 24 10       	mov    0x10(%rsp),%r15
  40c20c:	48 8b 5c 24 18       	mov    0x18(%rsp),%rbx
  40c211:	48 85 db             	test   %rbx,%rbx
  40c214:	74 2c                	je     40c242 <pt_str_new+0x82>
  40c216:	ff 15 f4 37 05 00    	call   *0x537f4(%rip)        # 45fa10 <_DYNAMIC+0x488>
  40c21c:	be 01 00 00 00       	mov    $0x1,%esi
  40c221:	48 89 df             	mov    %rbx,%rdi
  40c224:	ff 15 26 38 05 00    	call   *0x53826(%rip)        # 45fa50 <_DYNAMIC+0x4c8>
  40c22a:	49 89 c6             	mov    %rax,%r14
  40c22d:	48 85 c0             	test   %rax,%rax
  40c230:	75 16                	jne    40c248 <pt_str_new+0x88>
  40c232:	bf 01 00 00 00       	mov    $0x1,%edi
  40c237:	48 89 de             	mov    %rbx,%rsi
  40c23a:	ff 15 10 3a 05 00    	call   *0x53a10(%rip)        # 45fc50 <_DYNAMIC+0x6c8>
  40c240:	eb 72                	jmp    40c2b4 <pt_str_new+0xf4>
  40c242:	41 be 01 00 00 00    	mov    $0x1,%r14d
  40c248:	4c 89 f7             	mov    %r14,%rdi
  40c24b:	4c 89 fe             	mov    %r15,%rsi
  40c24e:	48 89 da             	mov    %rbx,%rdx
  40c251:	ff 15 31 3d 05 00    	call   *0x53d31(%rip)        # 45ff88 <memcpy@GLIBC_2.14>
  40c257:	48 83 fb 20          	cmp    $0x20,%rbx
  40c25b:	72 9c                	jb     40c1f9 <pt_str_new+0x39>
  40c25d:	4c 89 f7             	mov    %r14,%rdi
  40c260:	48 89 de             	mov    %rbx,%rsi
  40c263:	ff 15 57 3c 05 00    	call   *0x53c57(%rip)        # 45fec0 <_DYNAMIC+0x938>
  40c269:	49 89 c7             	mov    %rax,%r15
  40c26c:	ff 15 9e 37 05 00    	call   *0x5379e(%rip)        # 45fa10 <_DYNAMIC+0x488>
  40c272:	bf 20 00 00 00       	mov    $0x20,%edi
  40c277:	be 08 00 00 00       	mov    $0x8,%esi
  40c27c:	ff 15 ce 37 05 00    	call   *0x537ce(%rip)        # 45fa50 <_DYNAMIC+0x4c8>
  40c282:	48 85 c0             	test   %rax,%rax
  40c285:	74 1d                	je     40c2a4 <pt_str_new+0xe4>
  40c287:	4c 89 30             	mov    %r14,(%rax)
  40c28a:	48 89 58 08          	mov    %rbx,0x8(%rax)
  40c28e:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
  40c295:	00 
  40c296:	4c 89 78 18          	mov    %r15,0x18(%rax)
  40c29a:	48 83 c4 20          	add    $0x20,%rsp
  40c29e:	5b                   	pop    %rbx
  40c29f:	41 5e                	pop    %r14
  40c2a1:	41 5f                	pop    %r15
  40c2a3:	c3                   	ret
  40c2a4:	bf 08 00 00 00       	mov    $0x8,%edi
  40c2a9:	be 20 00 00 00       	mov    $0x20,%esi
  40c2ae:	ff 15 9c 39 05 00    	call   *0x5399c(%rip)        # 45fc50 <_DYNAMIC+0x6c8>
  40c2b4:	0f 0b                	ud2
  40c2b6:	eb 19                	jmp    40c2d1 <pt_str_new+0x111>
  40c2b8:	eb 0c                	jmp    40c2c6 <pt_str_new+0x106>
  40c2ba:	ff 15 e0 35 05 00    	call   *0x535e0(%rip)        # 45f8a0 <_DYNAMIC+0x318>
  40c2c0:	ff 15 da 35 05 00    	call   *0x535da(%rip)        # 45f8a0 <_DYNAMIC+0x318>
  40c2c6:	48 85 db             	test   %rbx,%rbx
  40c2c9:	75 06                	jne    40c2d1 <pt_str_new+0x111>
  40c2cb:	ff 15 cf 35 05 00    	call   *0x535cf(%rip)        # 45f8a0 <_DYNAMIC+0x318>
  40c2d1:	ba 01 00 00 00       	mov    $0x1,%edx
  40c2d6:	4c 89 f7             	mov    %r14,%rdi
  40c2d9:	48 89 de             	mov    %rbx,%rsi
  40c2dc:	ff 15 46 3c 05 00    	call   *0x53c46(%rip)        # 45ff28 <_DYNAMIC+0x9a0>
  40c2e2:	ff 15 b8 35 05 00    	call   *0x535b8(%rip)        # 45f8a0 <_DYNAMIC+0x318>

Disassembly of section .fini:

/workspace/card817/probe-scoped/profile:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .text:

000000000040bf30 <pt_str_index>:
  40bf30:	41 57                	push   %r15
  40bf32:	41 56                	push   %r14
  40bf34:	41 54                	push   %r12
  40bf36:	53                   	push   %rbx
  40bf37:	50                   	push   %rax
  40bf38:	48 8b 47 18          	mov    0x18(%rdi),%rax
  40bf3c:	48 85 c0             	test   %rax,%rax
  40bf3f:	0f 88 21 01 00 00    	js     40c066 <pt_str_index+0x136>
  40bf45:	49 89 f4             	mov    %rsi,%r12
  40bf48:	49 c1 fc 3f          	sar    $0x3f,%r12
  40bf4c:	49 21 c4             	and    %rax,%r12
  40bf4f:	49 01 f4             	add    %rsi,%r12
  40bf52:	0f 99 c1             	setns  %cl
  40bf55:	49 39 c4             	cmp    %rax,%r12
  40bf58:	0f 92 c2             	setb   %dl
  40bf5b:	20 ca                	and    %cl,%dl
  40bf5d:	80 fa 01             	cmp    $0x1,%dl
  40bf60:	0f 85 00 01 00 00    	jne    40c066 <pt_str_index+0x136>
  40bf66:	48 8b 1f             	mov    (%rdi),%rbx
  40bf69:	48 8b 77 08          	mov    0x8(%rdi),%rsi
  40bf6d:	48 39 f0             	cmp    %rsi,%rax
  40bf70:	75 33                	jne    40bfa5 <pt_str_index+0x75>
  40bf72:	4d 85 e4             	test   %r12,%r12
  40bf75:	74 0b                	je     40bf82 <pt_str_index+0x52>
  40bf77:	42 80 3c 23 bf       	cmpb   $0xbf,(%rbx,%r12,1)
  40bf7c:	0f 8e e4 00 00 00    	jle    40c066 <pt_str_index+0x136>
  40bf82:	49 8d 4c 24 01       	lea    0x1(%r12),%rcx
  40bf87:	41 be 01 00 00 00    	mov    $0x1,%r14d
  40bf8d:	48 39 c1             	cmp    %rax,%rcx
  40bf90:	0f 83 ca 00 00 00    	jae    40c060 <pt_str_index+0x130>
  40bf96:	80 3c 0b bf          	cmpb   $0xbf,(%rbx,%rcx,1)
  40bf9a:	0f 8f 4a 01 00 00    	jg     40c0ea <pt_str_index+0x1ba>
  40bfa0:	e9 c1 00 00 00       	jmp    40c066 <pt_str_index+0x136>
  40bfa5:	48 8d 0c 33          	lea    (%rbx,%rsi,1),%rcx
  40bfa9:	4d 85 e4             	test   %r12,%r12
  40bfac:	74 56                	je     40c004 <pt_str_index+0xd4>
  40bfae:	31 c0                	xor    %eax,%eax
  40bfb0:	31 d2                	xor    %edx,%edx
  40bfb2:	49 89 d8             	mov    %rbx,%r8
  40bfb5:	eb 1e                	jmp    40bfd5 <pt_str_index+0xa5>
  40bfb7:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
  40bfbe:	00 00 
  40bfc0:	49 8d 78 01          	lea    0x1(%r8),%rdi
  40bfc4:	49 89 f9             	mov    %rdi,%r9
  40bfc7:	4d 29 c1             	sub    %r8,%r9
  40bfca:	4c 01 ca             	add    %r9,%rdx
  40bfcd:	49 89 f8             	mov    %rdi,%r8
  40bfd0:	49 ff cc             	dec    %r12
  40bfd3:	74 34                	je     40c009 <pt_str_index+0xd9>
  40bfd5:	49 39 c8             	cmp    %rcx,%r8
  40bfd8:	0f 84 8a 00 00 00    	je     40c068 <pt_str_index+0x138>
  40bfde:	41 0f b6 38          	movzbl (%r8),%edi
  40bfe2:	40 84 ff             	test   %dil,%dil
  40bfe5:	79 d9                	jns    40bfc0 <pt_str_index+0x90>
  40bfe7:	40 80 ff e0          	cmp    $0xe0,%dil
  40bfeb:	72 11                	jb     40bffe <pt_str_index+0xce>
  40bfed:	40 80 ff f0          	cmp    $0xf0,%dil
  40bff1:	4c 89 c7             	mov    %r8,%rdi
  40bff4:	48 83 df ff          	sbb    $0xffffffffffffffff,%rdi
  40bff8:	48 83 c7 03          	add    $0x3,%rdi
  40bffc:	eb c6                	jmp    40bfc4 <pt_str_index+0x94>
  40bffe:	49 8d 78 02          	lea    0x2(%r8),%rdi
  40c002:	eb c0                	jmp    40bfc4 <pt_str_index+0x94>
  40c004:	31 d2                	xor    %edx,%edx
  40c006:	48 89 df             	mov    %rbx,%rdi
  40c009:	48 39 cf             	cmp    %rcx,%rdi
  40c00c:	74 58                	je     40c066 <pt_str_index+0x136>
  40c00e:	44 0f b6 07          	movzbl (%rdi),%r8d
  40c012:	41 be 01 00 00 00    	mov    $0x1,%r14d
  40c018:	45 84 c0             	test   %r8b,%r8b
  40c01b:	0f 89 83 00 00 00    	jns    40c0a4 <pt_str_index+0x174>
  40c021:	44 89 c0             	mov    %r8d,%eax
  40c024:	83 e0 1f             	and    $0x1f,%eax
  40c027:	44 0f b6 4f 01       	movzbl 0x1(%rdi),%r9d
  40c02c:	41 83 e1 3f          	and    $0x3f,%r9d
  40c030:	41 80 f8 e0          	cmp    $0xe0,%r8b
  40c034:	72 3e                	jb     40c074 <pt_str_index+0x144>
  40c036:	0f b6 4f 02          	movzbl 0x2(%rdi),%ecx
  40c03a:	41 c1 e1 06          	shl    $0x6,%r9d
  40c03e:	83 e1 3f             	and    $0x3f,%ecx
  40c041:	44 09 c9             	or     %r9d,%ecx
  40c044:	41 80 f8 f0          	cmp    $0xf0,%r8b
  40c048:	72 32                	jb     40c07c <pt_str_index+0x14c>
  40c04a:	0f b6 7f 03          	movzbl 0x3(%rdi),%edi
  40c04e:	83 e0 07             	and    $0x7,%eax
  40c051:	c1 e0 12             	shl    $0x12,%eax
  40c054:	c1 e1 06             	shl    $0x6,%ecx
  40c057:	83 e7 3f             	and    $0x3f,%edi
  40c05a:	09 cf                	or     %ecx,%edi
  40c05c:	09 f8                	or     %edi,%eax
  40c05e:	eb 21                	jmp    40c081 <pt_str_index+0x151>
  40c060:	0f 84 84 00 00 00    	je     40c0ea <pt_str_index+0x1ba>
  40c066:	31 c0                	xor    %eax,%eax
  40c068:	48 83 c4 08          	add    $0x8,%rsp
  40c06c:	5b                   	pop    %rbx
  40c06d:	41 5c                	pop    %r12
  40c06f:	41 5e                	pop    %r14
  40c071:	41 5f                	pop    %r15
  40c073:	c3                   	ret
  40c074:	c1 e0 06             	shl    $0x6,%eax
  40c077:	44 09 c8             	or     %r9d,%eax
  40c07a:	eb 05                	jmp    40c081 <pt_str_index+0x151>
  40c07c:	c1 e0 0c             	shl    $0xc,%eax
  40c07f:	09 c8                	or     %ecx,%eax
  40c081:	3d 80 00 00 00       	cmp    $0x80,%eax
  40c086:	72 1c                	jb     40c0a4 <pt_str_index+0x174>
  40c088:	41 be 02 00 00 00    	mov    $0x2,%r14d
  40c08e:	3d 00 08 00 00       	cmp    $0x800,%eax
  40c093:	72 0f                	jb     40c0a4 <pt_str_index+0x174>
  40c095:	3d 00 00 01 00       	cmp    $0x10000,%eax
  40c09a:	41 be 04 00 00 00    	mov    $0x4,%r14d
  40c0a0:	49 83 de 00          	sbb    $0x0,%r14
  40c0a4:	4c 89 f1             	mov    %r14,%rcx
  40c0a7:	48 01 d1             	add    %rdx,%rcx
  40c0aa:	72 24                	jb     40c0d0 <pt_str_index+0x1a0>
  40c0ac:	48 85 d2             	test   %rdx,%rdx
  40c0af:	74 0f                	je     40c0c0 <pt_str_index+0x190>
  40c0b1:	48 39 f2             	cmp    %rsi,%rdx
  40c0b4:	73 08                	jae    40c0be <pt_str_index+0x18e>
  40c0b6:	80 3c 13 bf          	cmpb   $0xbf,(%rbx,%rdx,1)
  40c0ba:	7f 04                	jg     40c0c0 <pt_str_index+0x190>
  40c0bc:	eb 12                	jmp    40c0d0 <pt_str_index+0x1a0>
  40c0be:	75 10                	jne    40c0d0 <pt_str_index+0x1a0>
  40c0c0:	48 85 c9             	test   %rcx,%rcx
  40c0c3:	74 22                	je     40c0e7 <pt_str_index+0x1b7>
  40c0c5:	48 39 f1             	cmp    %rsi,%rcx
  40c0c8:	73 1b                	jae    40c0e5 <pt_str_index+0x1b5>
  40c0ca:	80 3c 0b bf          	cmpb   $0xbf,(%rbx,%rcx,1)
  40c0ce:	7f 17                	jg     40c0e7 <pt_str_index+0x1b7>
  40c0d0:	4c 8d 05 11 13 05 00 	lea    0x51311(%rip),%r8        # 45d3e8 <__do_global_dtors_aux_fini_array_entry+0xf8>
  40c0d7:	48 89 df             	mov    %rbx,%rdi
  40c0da:	ff 15 d8 36 05 00    	call   *0x536d8(%rip)        # 45f7b8 <_DYNAMIC+0x230>
  40c0e0:	e9 95 00 00 00       	jmp    40c17a <pt_str_index+0x24a>
  40c0e5:	75 e9                	jne    40c0d0 <pt_str_index+0x1a0>
  40c0e7:	49 89 d4             	mov    %rdx,%r12
  40c0ea:	ff 15 20 39 05 00    	call   *0x53920(%rip)        # 45fa10 <_DYNAMIC+0x488>
  40c0f0:	be 01 00 00 00       	mov    $0x1,%esi
  40c0f5:	4c 89 f7             	mov    %r14,%rdi
  40c0f8:	ff 15 52 39 05 00    	call   *0x53952(%rip)        # 45fa50 <_DYNAMIC+0x4c8>
  40c0fe:	48 85 c0             	test   %rax,%rax
  40c101:	74 69                	je     40c16c <pt_str_index+0x23c>
  40c103:	49 89 c7             	mov    %rax,%r15
  40c106:	4c 01 e3             	add    %r12,%rbx
  40c109:	48 89 c7             	mov    %rax,%rdi
  40c10c:	48 89 de             	mov    %rbx,%rsi
  40c10f:	4c 89 f2             	mov    %r14,%rdx
  40c112:	ff 15 70 3e 05 00    	call   *0x53e70(%rip)        # 45ff88 <memcpy@GLIBC_2.14>
  40c118:	4c 89 ff             	mov    %r15,%rdi
  40c11b:	4c 89 f6             	mov    %r14,%rsi
  40c11e:	ff 15 dc 3b 05 00    	call   *0x53bdc(%rip)        # 45fd00 <_DYNAMIC+0x778>
  40c124:	48 89 c3             	mov    %rax,%rbx
  40c127:	ff 15 e3 38 05 00    	call   *0x538e3(%rip)        # 45fa10 <_DYNAMIC+0x488>
  40c12d:	bf 20 00 00 00       	mov    $0x20,%edi
  40c132:	be 08 00 00 00       	mov    $0x8,%esi
  40c137:	ff 15 13 39 05 00    	call   *0x53913(%rip)        # 45fa50 <_DYNAMIC+0x4c8>
  40c13d:	48 85 c0             	test   %rax,%rax
  40c140:	74 18                	je     40c15a <pt_str_index+0x22a>
  40c142:	4c 89 38             	mov    %r15,(%rax)
  40c145:	4c 89 70 08          	mov    %r14,0x8(%rax)
  40c149:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
  40c150:	00 
  40c151:	48 89 58 18          	mov    %rbx,0x18(%rax)
  40c155:	e9 0e ff ff ff       	jmp    40c068 <pt_str_index+0x138>
  40c15a:	bf 08 00 00 00       	mov    $0x8,%edi
  40c15f:	be 20 00 00 00       	mov    $0x20,%esi
  40c164:	ff 15 e6 3a 05 00    	call   *0x53ae6(%rip)        # 45fc50 <_DYNAMIC+0x6c8>
  40c16a:	eb 0e                	jmp    40c17a <pt_str_index+0x24a>
  40c16c:	bf 01 00 00 00       	mov    $0x1,%edi
  40c171:	4c 89 f6             	mov    %r14,%rsi
  40c174:	ff 15 d6 3a 05 00    	call   *0x53ad6(%rip)        # 45fc50 <_DYNAMIC+0x6c8>
  40c17a:	0f 0b                	ud2
  40c17c:	eb 06                	jmp    40c184 <pt_str_index+0x254>
  40c17e:	ff 15 1c 37 05 00    	call   *0x5371c(%rip)        # 45f8a0 <_DYNAMIC+0x318>
  40c184:	ba 01 00 00 00       	mov    $0x1,%edx
  40c189:	4c 89 ff             	mov    %r15,%rdi
  40c18c:	4c 89 f6             	mov    %r14,%rsi
  40c18f:	ff 15 93 3d 05 00    	call   *0x53d93(%rip)        # 45ff28 <_DYNAMIC+0x9a0>
  40c195:	ff 15 05 37 05 00    	call   *0x53705(%rip)        # 45f8a0 <_DYNAMIC+0x318>

Disassembly of section .fini:
