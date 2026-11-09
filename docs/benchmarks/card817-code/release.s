
/workspace/card817/baseline/strings:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000b770 <pythrust_main>:
    b770:	55                   	push   %rbp
    b771:	48 89 e5             	mov    %rsp,%rbp
    b774:	48 81 ec 80 00 00 00 	sub    $0x80,%rsp
    b77b:	48 89 5c 24 50       	mov    %rbx,0x50(%rsp)
    b780:	4c 89 64 24 58       	mov    %r12,0x58(%rsp)
    b785:	4c 89 6c 24 60       	mov    %r13,0x60(%rsp)
    b78a:	4c 89 74 24 68       	mov    %r14,0x68(%rsp)
    b78f:	4c 89 7c 24 70       	mov    %r15,0x70(%rsp)
    b794:	48 8d 3d 65 38 04 00 	lea    0x43865(%rip),%rdi        # 4f000 <.Ldata1>
    b79b:	48 33 db             	xor    %rbx,%rbx
    b79e:	48 8d 05 bb 49 00 00 	lea    0x49bb(%rip),%rax        # 10160 <pt_str_new>
    b7a5:	48 89 de             	mov    %rbx,%rsi
    b7a8:	ff d0                	call   *%rax
    b7aa:	4c 8d 25 4f 38 04 00 	lea    0x4384f(%rip),%r12        # 4f000 <.Ldata1>
    b7b1:	4c 8d 2d 49 38 04 00 	lea    0x43849(%rip),%r13        # 4f001 <.Ldata2>
    b7b8:	49 89 c7             	mov    %rax,%r15
    b7bb:	48 89 5c 24 10       	mov    %rbx,0x10(%rsp)
    b7c0:	48 89 5c 24 18       	mov    %rbx,0x18(%rsp)
    b7c5:	48 81 fb 40 42 0f 00 	cmp    $0xf4240,%rbx
    b7cc:	0f 8d 40 02 00 00    	jge    ba12 <pythrust_main+0x2a2>
    b7d2:	41 be 01 00 00 00    	mov    $0x1,%r14d
    b7d8:	48 8d 05 81 49 00 00 	lea    0x4981(%rip),%rax        # 10160 <pt_str_new>
    b7df:	4c 89 f6             	mov    %r14,%rsi
    b7e2:	4c 89 e7             	mov    %r12,%rdi
    b7e5:	ff d0                	call   *%rax
    b7e7:	48 89 44 24 48       	mov    %rax,0x48(%rsp)
    b7ec:	48 8d 05 6d 40 00 00 	lea    0x406d(%rip),%rax        # f860 <pt_str_from_int>
    b7f3:	48 89 df             	mov    %rbx,%rdi
    b7f6:	ff d0                	call   *%rax
    b7f8:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
    b7fd:	48 8d 05 9c 3e 00 00 	lea    0x3e9c(%rip),%rax        # f6a0 <pt_str_concat>
    b804:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    b809:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    b80e:	ff d0                	call   *%rax
    b810:	48 89 44 24 38       	mov    %rax,0x38(%rsp)
    b815:	48 8d 05 44 49 00 00 	lea    0x4944(%rip),%rax        # 10160 <pt_str_new>
    b81c:	4c 89 f6             	mov    %r14,%rsi
    b81f:	4c 89 ef             	mov    %r13,%rdi
    b822:	ff d0                	call   *%rax
    b824:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
    b829:	48 8d 0d 70 3e 00 00 	lea    0x3e70(%rip),%rcx        # f6a0 <pt_str_concat>
    b830:	48 8b 74 24 30       	mov    0x30(%rsp),%rsi
    b835:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    b83a:	ff d1                	call   *%rcx
    b83c:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
    b841:	48 89 d8             	mov    %rbx,%rax
    b844:	48 f7 2d f5 02 00 00 	imulq  0x2f5(%rip)        # bb40 <pythrust_main+0x3d0>
    b84b:	48 8d 04 1a          	lea    (%rdx,%rbx,1),%rax
    b84f:	48 c1 f8 06          	sar    $0x6,%rax
    b853:	48 89 c1             	mov    %rax,%rcx
    b856:	48 c1 e9 3f          	shr    $0x3f,%rcx
    b85a:	48 03 c1             	add    %rcx,%rax
    b85d:	48 6b c8 61          	imul   $0x61,%rax,%rcx
    b861:	48 89 d8             	mov    %rbx,%rax
    b864:	48 2b c1             	sub    %rcx,%rax
    b867:	48 3b d9             	cmp    %rcx,%rbx
    b86a:	0f 95 c1             	setne  %cl
    b86d:	48 89 c6             	mov    %rax,%rsi
    b870:	48 83 f6 61          	xor    $0x61,%rsi
    b874:	48 33 d2             	xor    %rdx,%rdx
    b877:	48 c1 ee 3f          	shr    $0x3f,%rsi
    b87b:	40 84 ce             	test   %cl,%sil
    b87e:	48 0f 45 15 c2 02 00 	cmovne 0x2c2(%rip),%rdx        # bb48 <pythrust_main+0x3d8>
    b885:	00 
    b886:	48 8d 3c 10          	lea    (%rax,%rdx,1),%rdi
    b88a:	48 8d 05 cf 3f 00 00 	lea    0x3fcf(%rip),%rax        # f860 <pt_str_from_int>
    b891:	ff d0                	call   *%rax
    b893:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
    b898:	48 8d 0d 01 3e 00 00 	lea    0x3e01(%rip),%rcx        # f6a0 <pt_str_concat>
    b89f:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
    b8a4:	48 8b 7c 24 20       	mov    0x20(%rsp),%rdi
    b8a9:	ff d1                	call   *%rcx
    b8ab:	49 89 c6             	mov    %rax,%r14
    b8ae:	48 8d 05 2b 3f 00 00 	lea    0x3f2b(%rip),%rax        # f7e0 <pt_str_dec>
    b8b5:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
    b8ba:	ff d0                	call   *%rax
    b8bc:	48 8d 05 1d 3f 00 00 	lea    0x3f1d(%rip),%rax        # f7e0 <pt_str_dec>
    b8c3:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    b8c8:	ff d0                	call   *%rax
    b8ca:	48 8d 05 0f 3f 00 00 	lea    0x3f0f(%rip),%rax        # f7e0 <pt_str_dec>
    b8d1:	48 8b 7c 24 40       	mov    0x40(%rsp),%rdi
    b8d6:	ff d0                	call   *%rax
    b8d8:	48 8d 05 01 3f 00 00 	lea    0x3f01(%rip),%rax        # f7e0 <pt_str_dec>
    b8df:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    b8e4:	ff d0                	call   *%rax
    b8e6:	48 8d 05 f3 3e 00 00 	lea    0x3ef3(%rip),%rax        # f7e0 <pt_str_dec>
    b8ed:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
    b8f2:	ff d0                	call   *%rax
    b8f4:	48 8d 0d e5 3e 00 00 	lea    0x3ee5(%rip),%rcx        # f7e0 <pt_str_dec>
    b8fb:	48 8b 7c 24 20       	mov    0x20(%rsp),%rdi
    b900:	ff d1                	call   *%rcx
    b902:	48 8d 15 d7 3e 00 00 	lea    0x3ed7(%rip),%rdx        # f7e0 <pt_str_dec>
    b909:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
    b90e:	ff d2                	call   *%rdx
    b910:	48 8d 35 29 48 00 00 	lea    0x4829(%rip),%rsi        # 10140 <pt_str_len>
    b917:	4c 89 f7             	mov    %r14,%rdi
    b91a:	ff d6                	call   *%rsi
    b91c:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
    b921:	48 03 f8             	add    %rax,%rdi
    b924:	48 89 7c 24 10       	mov    %rdi,0x10(%rsp)
    b929:	40 0f 90 c6          	seto   %sil
    b92d:	40 84 f6             	test   %sil,%sil
    b930:	0f 85 ef 01 00 00    	jne    bb25 <pythrust_main+0x3b5>
    b936:	48 89 d8             	mov    %rbx,%rax
    b939:	48 f7 2d 10 02 00 00 	imulq  0x210(%rip)        # bb50 <pythrust_main+0x3e0>
    b940:	48 8d 34 1a          	lea    (%rdx,%rbx,1),%rsi
    b944:	48 c1 fe 05          	sar    $0x5,%rsi
    b948:	48 89 f7             	mov    %rsi,%rdi
    b94b:	48 c1 ef 3f          	shr    $0x3f,%rdi
    b94f:	48 03 f7             	add    %rdi,%rsi
    b952:	48 6b fe 32          	imul   $0x32,%rsi,%rdi
    b956:	48 89 de             	mov    %rbx,%rsi
    b959:	48 2b f7             	sub    %rdi,%rsi
    b95c:	48 3b df             	cmp    %rdi,%rbx
    b95f:	40 0f 95 c7          	setne  %dil
    b963:	49 89 f1             	mov    %rsi,%r9
    b966:	49 83 f1 32          	xor    $0x32,%r9
    b96a:	4d 33 c0             	xor    %r8,%r8
    b96d:	49 c1 e9 3f          	shr    $0x3f,%r9
    b971:	41 84 f9             	test   %dil,%r9b
    b974:	4c 0f 45 05 dc 01 00 	cmovne 0x1dc(%rip),%r8        # bb58 <pythrust_main+0x3e8>
    b97b:	00 
    b97c:	4c 03 c6             	add    %rsi,%r8
    b97f:	4d 85 c0             	test   %r8,%r8
    b982:	0f 85 55 00 00 00    	jne    b9dd <pythrust_main+0x26d>
    b988:	48 33 f6             	xor    %rsi,%rsi
    b98b:	48 8d 05 3e 45 00 00 	lea    0x453e(%rip),%rax        # fed0 <pt_str_index>
    b992:	4c 89 f7             	mov    %r14,%rdi
    b995:	ff d0                	call   *%rax
    b997:	48 89 c2             	mov    %rax,%rdx
    b99a:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
    b99f:	48 85 d0             	test   %rdx,%rax
    b9a2:	0f 84 12 01 00 00    	je     baba <pythrust_main+0x34a>
    b9a8:	48 8d 0d f1 3c 00 00 	lea    0x3cf1(%rip),%rcx        # f6a0 <pt_str_concat>
    b9af:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
    b9b4:	4c 89 ff             	mov    %r15,%rdi
    b9b7:	ff d1                	call   *%rcx
    b9b9:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
    b9be:	48 8d 05 1b 3e 00 00 	lea    0x3e1b(%rip),%rax        # f7e0 <pt_str_dec>
    b9c5:	4c 89 ff             	mov    %r15,%rdi
    b9c8:	ff d0                	call   *%rax
    b9ca:	48 8d 05 0f 3e 00 00 	lea    0x3e0f(%rip),%rax        # f7e0 <pt_str_dec>
    b9d1:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
    b9d6:	ff d0                	call   *%rax
    b9d8:	4c 8b 7c 24 20       	mov    0x20(%rsp),%r15
    b9dd:	48 89 d8             	mov    %rbx,%rax
    b9e0:	48 83 c0 01          	add    $0x1,%rax
    b9e4:	0f 90 c0             	seto   %al
    b9e7:	84 c0                	test   %al,%al
    b9e9:	0f 84 0a 00 00 00    	je     b9f9 <pythrust_main+0x289>
    b9ef:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    b9f4:	e9 19 00 00 00       	jmp    ba12 <pythrust_main+0x2a2>
    b9f9:	48 83 c3 01          	add    $0x1,%rbx
    b9fd:	0f 90 c0             	seto   %al
    ba00:	84 c0                	test   %al,%al
    ba02:	0f 85 9b 00 00 00    	jne    baa3 <pythrust_main+0x333>
    ba08:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    ba0d:	e9 b3 fd ff ff       	jmp    b7c5 <pythrust_main+0x55>
    ba12:	48 8d 05 27 47 00 00 	lea    0x4727(%rip),%rax        # 10140 <pt_str_len>
    ba19:	4c 89 ff             	mov    %r15,%rdi
    ba1c:	ff d0                	call   *%rax
    ba1e:	48 89 c3             	mov    %rax,%rbx
    ba21:	48 8d 05 b8 6d 00 00 	lea    0x6db8(%rip),%rax        # 127e0 <pt_write_int>
    ba28:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
    ba2d:	ff d0                	call   *%rax
    ba2f:	b8 20 00 00 00       	mov    $0x20,%eax
    ba34:	48 0f b6 f8          	movzbq %al,%rdi
    ba38:	48 8d 05 71 6d 00 00 	lea    0x6d71(%rip),%rax        # 127b0 <pt_write_byte>
    ba3f:	ff d0                	call   *%rax
    ba41:	48 8d 05 98 6d 00 00 	lea    0x6d98(%rip),%rax        # 127e0 <pt_write_int>
    ba48:	48 89 df             	mov    %rbx,%rdi
    ba4b:	ff d0                	call   *%rax
    ba4d:	b8 0a 00 00 00       	mov    $0xa,%eax
    ba52:	48 0f b6 f8          	movzbq %al,%rdi
    ba56:	48 8d 05 53 6d 00 00 	lea    0x6d53(%rip),%rax        # 127b0 <pt_write_byte>
    ba5d:	ff d0                	call   *%rax
    ba5f:	48 8d 05 7a 3d 00 00 	lea    0x3d7a(%rip),%rax        # f7e0 <pt_str_dec>
    ba66:	4c 89 ff             	mov    %r15,%rdi
    ba69:	ff d0                	call   *%rax
    ba6b:	48 8d 05 6e 3d 00 00 	lea    0x3d6e(%rip),%rax        # f7e0 <pt_str_dec>
    ba72:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
    ba77:	ff d0                	call   *%rax
    ba79:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
    ba7e:	4c 8b 64 24 58       	mov    0x58(%rsp),%r12
    ba83:	4c 8b 6c 24 60       	mov    0x60(%rsp),%r13
    ba88:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
    ba8d:	4c 8b 7c 24 70       	mov    0x70(%rsp),%r15
    ba92:	48 81 c4 80 00 00 00 	add    $0x80,%rsp
    ba99:	48 89 ec             	mov    %rbp,%rsp
    ba9c:	5d                   	pop    %rbp
    ba9d:	c3                   	ret
    ba9e:	e9 53 00 00 00       	jmp    baf6 <pythrust_main+0x386>
    baa3:	48 8d 3d fa 36 04 00 	lea    0x436fa(%rip),%rdi        # 4f1a4 <.Ldata9>
    baaa:	be 90 00 00 00       	mov    $0x90,%esi
    baaf:	48 8d 05 9a 6c 00 00 	lea    0x6c9a(%rip),%rax        # 12750 <pt_fail>
    bab6:	ff d0                	call   *%rax
    bab8:	0f 0b                	ud2
    baba:	b8 01 00 00 00       	mov    $0x1,%eax
    babf:	48 8d 35 fd 37 04 00 	lea    0x437fd(%rip),%rsi        # 4f2c3 <.Ldatab>
    bac6:	ba 19 00 00 00       	mov    $0x19,%edx
    bacb:	48 8d 0d 0a 38 04 00 	lea    0x4380a(%rip),%rcx        # 4f2dc <.Ldatac>
    bad2:	41 b8 3b 00 00 00    	mov    $0x3b,%r8d
    bad8:	4c 8d 0d 38 38 04 00 	lea    0x43838(%rip),%r9        # 4f317 <.Ldatad>
    badf:	41 ba 33 00 00 00    	mov    $0x33,%r10d
    bae5:	48 0f b6 f8          	movzbq %al,%rdi
    bae9:	4c 89 14 24          	mov    %r10,(%rsp)
    baed:	48 8d 05 4c 75 00 00 	lea    0x754c(%rip),%rax        # 13040 <pt_raise_msg>
    baf4:	ff d0                	call   *%rax
    baf6:	48 8d 05 e3 3c 00 00 	lea    0x3ce3(%rip),%rax        # f7e0 <pt_str_dec>
    bafd:	4c 89 ff             	mov    %r15,%rdi
    bb00:	ff d0                	call   *%rax
    bb02:	48 8d 05 d7 3c 00 00 	lea    0x3cd7(%rip),%rax        # f7e0 <pt_str_dec>
    bb09:	4c 89 f7             	mov    %r14,%rdi
    bb0c:	ff d0                	call   *%rax
    bb0e:	48 33 ff             	xor    %rdi,%rdi
    bb11:	48 8d 05 c8 3c 00 00 	lea    0x3cc8(%rip),%rax        # f7e0 <pt_str_dec>
    bb18:	ff d0                	call   *%rax
    bb1a:	48 8d 05 0f 76 00 00 	lea    0x760f(%rip),%rax        # 13130 <pt_uncaught>
    bb21:	ff d0                	call   *%rax
    bb23:	0f 0b                	ud2
    bb25:	48 8d 3d 8d 35 04 00 	lea    0x4358d(%rip),%rdi        # 4f0b9 <.Ldata6>
    bb2c:	be 8a 00 00 00       	mov    $0x8a,%esi
    bb31:	48 8d 05 18 6c 00 00 	lea    0x6c18(%rip),%rax        # 12750 <pt_fail>
    bb38:	ff d0                	call   *%rax
    bb3a:	0f 0b                	ud2
    bb3c:	00 00                	add    %al,(%rax)
    bb3e:	00 00                	add    %al,(%rax)
    bb40:	e9 a8 c0 17 57       	jmp    57187bed <_end+0x57123115>
    bb45:	3f                   	(bad)
    bb46:	e8 a8 61 00 00       	call   11cf3 <_RINvNtCs40k4W9msRzi_5alloc3str17join_generic_copyehReECsk1kE6r7WBHz_11pythrust_rt+0x473>
    bb4b:	00 00                	add    %al,(%rax)
    bb4d:	00 00                	add    %al,(%rax)
    bb4f:	00 0b                	add    %cl,(%rbx)
    bb51:	d7                   	xlat   %ds:(%rbx)
    bb52:	a3 70 3d 0a d7 a3 32 	movabs %eax,0x32a3d70a3d70
    bb59:	00 00 
    bb5b:	00 00                	add    %al,(%rax)
    bb5d:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:

/workspace/card817/baseline/strings:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000f6a0 <pt_str_concat>:
    f6a0:	55                   	push   %rbp
    f6a1:	41 57                	push   %r15
    f6a3:	41 56                	push   %r14
    f6a5:	41 55                	push   %r13
    f6a7:	41 54                	push   %r12
    f6a9:	53                   	push   %rbx
    f6aa:	48 83 ec 38          	sub    $0x38,%rsp
    f6ae:	48 89 f3             	mov    %rsi,%rbx
    f6b1:	49 89 fe             	mov    %rdi,%r14
    f6b4:	0f 10 07             	movups (%rdi),%xmm0
    f6b7:	0f 10 0e             	movups (%rsi),%xmm1
    f6ba:	0f 11 44 24 18       	movups %xmm0,0x18(%rsp)
    f6bf:	0f 11 4c 24 28       	movups %xmm1,0x28(%rsp)
    f6c4:	48 89 e7             	mov    %rsp,%rdi
    f6c7:	48 8d 74 24 18       	lea    0x18(%rsp),%rsi
    f6cc:	ba 02 00 00 00       	mov    $0x2,%edx
    f6d1:	b9 01 00 00 00       	mov    $0x1,%ecx
    f6d6:	45 31 c0             	xor    %r8d,%r8d
    f6d9:	e8 a2 21 00 00       	call   11880 <_RINvNtCs40k4W9msRzi_5alloc3str17join_generic_copyehReECsk1kE6r7WBHz_11pythrust_rt>
    f6de:	49 8b 6e 18          	mov    0x18(%r14),%rbp
    f6e2:	4c 8b 6b 18          	mov    0x18(%rbx),%r13
    f6e6:	4c 8b 3c 24          	mov    (%rsp),%r15
    f6ea:	48 8b 5c 24 10       	mov    0x10(%rsp),%rbx
    f6ef:	49 39 df             	cmp    %rbx,%r15
    f6f2:	76 36                	jbe    f72a <pt_str_concat+0x8a>
    f6f4:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
    f6f9:	48 85 db             	test   %rbx,%rbx
    f6fc:	74 33                	je     f731 <pt_str_concat+0x91>
    f6fe:	ba 01 00 00 00       	mov    $0x1,%edx
    f703:	4c 89 e7             	mov    %r12,%rdi
    f706:	4c 89 fe             	mov    %r15,%rsi
    f709:	48 89 d9             	mov    %rbx,%rcx
    f70c:	ff 15 e6 45 05 00    	call   *0x545e6(%rip)        # 63cf8 <_DYNAMIC+0x798>
    f712:	49 89 c6             	mov    %rax,%r14
    f715:	48 85 c0             	test   %rax,%rax
    f718:	75 2e                	jne    f748 <pt_str_concat+0xa8>
    f71a:	bf 01 00 00 00       	mov    $0x1,%edi
    f71f:	48 89 de             	mov    %rbx,%rsi
    f722:	ff 15 b8 44 05 00    	call   *0x544b8(%rip)        # 63be0 <_DYNAMIC+0x680>
    f728:	eb 6e                	jmp    f798 <pt_str_concat+0xf8>
    f72a:	4c 8b 74 24 08       	mov    0x8(%rsp),%r14
    f72f:	eb 17                	jmp    f748 <pt_str_concat+0xa8>
    f731:	41 be 01 00 00 00    	mov    $0x1,%r14d
    f737:	ba 01 00 00 00       	mov    $0x1,%edx
    f73c:	4c 89 e7             	mov    %r12,%rdi
    f73f:	4c 89 fe             	mov    %r15,%rsi
    f742:	ff 15 e0 47 05 00    	call   *0x547e0(%rip)        # 63f28 <_DYNAMIC+0x9c8>
    f748:	ff 15 ba 42 05 00    	call   *0x542ba(%rip)        # 63a08 <_DYNAMIC+0x4a8>
    f74e:	bf 20 00 00 00       	mov    $0x20,%edi
    f753:	be 08 00 00 00       	mov    $0x8,%esi
    f758:	ff 15 ea 42 05 00    	call   *0x542ea(%rip)        # 63a48 <_DYNAMIC+0x4e8>
    f75e:	48 85 c0             	test   %rax,%rax
    f761:	74 25                	je     f788 <pt_str_concat+0xe8>
    f763:	49 01 ed             	add    %rbp,%r13
    f766:	4c 89 30             	mov    %r14,(%rax)
    f769:	48 89 58 08          	mov    %rbx,0x8(%rax)
    f76d:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
    f774:	00 
    f775:	4c 89 68 18          	mov    %r13,0x18(%rax)
    f779:	48 83 c4 38          	add    $0x38,%rsp
    f77d:	5b                   	pop    %rbx
    f77e:	41 5c                	pop    %r12
    f780:	41 5d                	pop    %r13
    f782:	41 5e                	pop    %r14
    f784:	41 5f                	pop    %r15
    f786:	5d                   	pop    %rbp
    f787:	c3                   	ret
    f788:	bf 08 00 00 00       	mov    $0x8,%edi
    f78d:	be 20 00 00 00       	mov    $0x20,%esi
    f792:	ff 15 b8 44 05 00    	call   *0x544b8(%rip)        # 63c50 <_DYNAMIC+0x6f0>
    f798:	0f 0b                	ud2
    f79a:	ba 01 00 00 00       	mov    $0x1,%edx
    f79f:	4c 89 e7             	mov    %r12,%rdi
    f7a2:	4c 89 fe             	mov    %r15,%rsi
    f7a5:	ff 15 7d 47 05 00    	call   *0x5477d(%rip)        # 63f28 <_DYNAMIC+0x9c8>
    f7ab:	ff 15 d7 40 05 00    	call   *0x540d7(%rip)        # 63888 <_DYNAMIC+0x328>
    f7b1:	ff 15 d1 40 05 00    	call   *0x540d1(%rip)        # 63888 <_DYNAMIC+0x328>
    f7b7:	48 85 db             	test   %rbx,%rbx
    f7ba:	75 06                	jne    f7c2 <pt_str_concat+0x122>
    f7bc:	ff 15 c6 40 05 00    	call   *0x540c6(%rip)        # 63888 <_DYNAMIC+0x328>
    f7c2:	ba 01 00 00 00       	mov    $0x1,%edx
    f7c7:	4c 89 f7             	mov    %r14,%rdi
    f7ca:	48 89 de             	mov    %rbx,%rsi
    f7cd:	ff 15 55 47 05 00    	call   *0x54755(%rip)        # 63f28 <_DYNAMIC+0x9c8>
    f7d3:	ff 15 af 40 05 00    	call   *0x540af(%rip)        # 63888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card817/baseline/strings:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000f860 <pt_str_from_int>:
    f860:	41 57                	push   %r15
    f862:	41 56                	push   %r14
    f864:	41 54                	push   %r12
    f866:	53                   	push   %rbx
    f867:	48 83 ec 38          	sub    $0x38,%rsp
    f86b:	0f 28 05 4e 30 04 00 	movaps 0x4304e(%rip),%xmm0        # 528c0 <anon.13df560d6b83da2c10641ad9b2156d4a.31.llvm.366253514071984098+0x55>
    f872:	0f 29 04 24          	movaps %xmm0,(%rsp)
    f876:	c7 44 24 0f 30 30 30 	movl   $0x30303030,0xf(%rsp)
    f87d:	30 
    f87e:	48 89 f9             	mov    %rdi,%rcx
    f881:	48 f7 d9             	neg    %rcx
    f884:	48 0f 48 cf          	cmovs  %rdi,%rcx
    f888:	48 ba cd cc cc cc cc 	movabs $0xcccccccccccccccd,%rdx
    f88f:	cc cc cc 
    f892:	48 89 c8             	mov    %rcx,%rax
    f895:	48 f7 e2             	mul    %rdx
    f898:	48 c1 ea 03          	shr    $0x3,%rdx
    f89c:	8d 04 12             	lea    (%rdx,%rdx,1),%eax
    f89f:	8d 04 80             	lea    (%rax,%rax,4),%eax
    f8a2:	89 ce                	mov    %ecx,%esi
    f8a4:	29 c6                	sub    %eax,%esi
    f8a6:	40 80 ce 30          	or     $0x30,%sil
    f8aa:	40 88 74 24 13       	mov    %sil,0x13(%rsp)
    f8af:	48 83 f9 0a          	cmp    $0xa,%rcx
    f8b3:	73 0f                	jae    f8c4 <pt_str_from_int+0x64>
    f8b5:	b8 13 00 00 00       	mov    $0x13,%eax
    f8ba:	b9 12 00 00 00       	mov    $0x12,%ecx
    f8bf:	e9 f1 04 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    f8c4:	49 b8 9a 99 99 99 99 	movabs $0x199999999999999a,%r8
    f8cb:	99 99 19 
    f8ce:	48 89 d0             	mov    %rdx,%rax
    f8d1:	48 89 d6             	mov    %rdx,%rsi
    f8d4:	49 f7 e0             	mul    %r8
    f8d7:	01 d2                	add    %edx,%edx
    f8d9:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
    f8dc:	29 c6                	sub    %eax,%esi
    f8de:	40 80 ce 30          	or     $0x30,%sil
    f8e2:	40 88 74 24 12       	mov    %sil,0x12(%rsp)
    f8e7:	48 83 f9 64          	cmp    $0x64,%rcx
    f8eb:	73 0f                	jae    f8fc <pt_str_from_int+0x9c>
    f8ed:	b8 12 00 00 00       	mov    $0x12,%eax
    f8f2:	b9 11 00 00 00       	mov    $0x11,%ecx
    f8f7:	e9 b9 04 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    f8fc:	48 89 c8             	mov    %rcx,%rax
    f8ff:	48 c1 e8 02          	shr    $0x2,%rax
    f903:	48 ba c3 f5 28 5c 8f 	movabs $0x28f5c28f5c28f5c3,%rdx
    f90a:	c2 f5 28 
    f90d:	48 f7 e2             	mul    %rdx
    f910:	48 89 d6             	mov    %rdx,%rsi
    f913:	48 c1 ee 02          	shr    $0x2,%rsi
    f917:	48 89 f0             	mov    %rsi,%rax
    f91a:	49 f7 e0             	mul    %r8
    f91d:	01 d2                	add    %edx,%edx
    f91f:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
    f922:	29 c6                	sub    %eax,%esi
    f924:	40 80 ce 30          	or     $0x30,%sil
    f928:	40 88 74 24 11       	mov    %sil,0x11(%rsp)
    f92d:	48 81 f9 e8 03 00 00 	cmp    $0x3e8,%rcx
    f934:	73 0f                	jae    f945 <pt_str_from_int+0xe5>
    f936:	b8 11 00 00 00       	mov    $0x11,%eax
    f93b:	b9 10 00 00 00       	mov    $0x10,%ecx
    f940:	e9 70 04 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    f945:	48 89 c8             	mov    %rcx,%rax
    f948:	48 c1 e8 03          	shr    $0x3,%rax
    f94c:	48 ba cf f7 53 e3 a5 	movabs $0x20c49ba5e353f7cf,%rdx
    f953:	9b c4 20 
    f956:	48 f7 e2             	mul    %rdx
    f959:	48 89 d6             	mov    %rdx,%rsi
    f95c:	48 c1 ee 04          	shr    $0x4,%rsi
    f960:	48 89 f0             	mov    %rsi,%rax
    f963:	49 f7 e0             	mul    %r8
    f966:	01 d2                	add    %edx,%edx
    f968:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
    f96b:	29 c6                	sub    %eax,%esi
    f96d:	40 80 ce 30          	or     $0x30,%sil
    f971:	40 88 74 24 10       	mov    %sil,0x10(%rsp)
    f976:	48 81 f9 10 27 00 00 	cmp    $0x2710,%rcx
    f97d:	73 0f                	jae    f98e <pt_str_from_int+0x12e>
    f97f:	b8 10 00 00 00       	mov    $0x10,%eax
    f984:	b9 0f 00 00 00       	mov    $0xf,%ecx
    f989:	e9 27 04 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    f98e:	48 ba 4b 59 86 38 d6 	movabs $0x346dc5d63886594b,%rdx
    f995:	c5 6d 34 
    f998:	48 89 c8             	mov    %rcx,%rax
    f99b:	48 f7 e2             	mul    %rdx
    f99e:	48 89 d6             	mov    %rdx,%rsi
    f9a1:	48 c1 ee 0b          	shr    $0xb,%rsi
    f9a5:	48 89 f0             	mov    %rsi,%rax
    f9a8:	49 f7 e0             	mul    %r8
    f9ab:	01 d2                	add    %edx,%edx
    f9ad:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
    f9b0:	29 c6                	sub    %eax,%esi
    f9b2:	40 80 ce 30          	or     $0x30,%sil
    f9b6:	40 88 74 24 0f       	mov    %sil,0xf(%rsp)
    f9bb:	48 81 f9 a0 86 01 00 	cmp    $0x186a0,%rcx
    f9c2:	73 0f                	jae    f9d3 <pt_str_from_int+0x173>
    f9c4:	b8 0f 00 00 00       	mov    $0xf,%eax
    f9c9:	b9 0e 00 00 00       	mov    $0xe,%ecx
    f9ce:	e9 e2 03 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    f9d3:	48 89 c8             	mov    %rcx,%rax
    f9d6:	48 c1 e8 05          	shr    $0x5,%rax
    f9da:	48 ba 43 78 b4 71 c4 	movabs $0xa7c5ac471b47843,%rdx
    f9e1:	5a 7c 0a 
    f9e4:	48 f7 e2             	mul    %rdx
    f9e7:	48 89 d6             	mov    %rdx,%rsi
    f9ea:	48 c1 ee 07          	shr    $0x7,%rsi
    f9ee:	48 89 f0             	mov    %rsi,%rax
    f9f1:	49 f7 e0             	mul    %r8
    f9f4:	01 d2                	add    %edx,%edx
    f9f6:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
    f9f9:	29 c6                	sub    %eax,%esi
    f9fb:	40 80 ce 30          	or     $0x30,%sil
    f9ff:	40 88 74 24 0e       	mov    %sil,0xe(%rsp)
    fa04:	48 81 f9 40 42 0f 00 	cmp    $0xf4240,%rcx
    fa0b:	73 0f                	jae    fa1c <pt_str_from_int+0x1bc>
    fa0d:	b8 0e 00 00 00       	mov    $0xe,%eax
    fa12:	b9 0d 00 00 00       	mov    $0xd,%ecx
    fa17:	e9 99 03 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    fa1c:	48 ba db 34 b6 d7 82 	movabs $0x431bde82d7b634db,%rdx
    fa23:	de 1b 43 
    fa26:	48 89 c8             	mov    %rcx,%rax
    fa29:	48 f7 e2             	mul    %rdx
    fa2c:	48 89 d6             	mov    %rdx,%rsi
    fa2f:	48 c1 ee 12          	shr    $0x12,%rsi
    fa33:	48 89 f0             	mov    %rsi,%rax
    fa36:	49 f7 e0             	mul    %r8
    fa39:	01 d2                	add    %edx,%edx
    fa3b:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
    fa3e:	29 c6                	sub    %eax,%esi
    fa40:	40 80 ce 30          	or     $0x30,%sil
    fa44:	40 88 74 24 0d       	mov    %sil,0xd(%rsp)
    fa49:	48 81 f9 80 96 98 00 	cmp    $0x989680,%rcx
    fa50:	73 0f                	jae    fa61 <pt_str_from_int+0x201>
    fa52:	b8 0d 00 00 00       	mov    $0xd,%eax
    fa57:	b9 0c 00 00 00       	mov    $0xc,%ecx
    fa5c:	e9 54 03 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    fa61:	48 ba bd 42 7a e5 d5 	movabs $0xd6bf94d5e57a42bd,%rdx
    fa68:	94 bf d6 
    fa6b:	48 89 c8             	mov    %rcx,%rax
    fa6e:	48 f7 e2             	mul    %rdx
    fa71:	48 89 d6             	mov    %rdx,%rsi
    fa74:	48 c1 ee 17          	shr    $0x17,%rsi
    fa78:	48 89 f0             	mov    %rsi,%rax
    fa7b:	49 f7 e0             	mul    %r8
    fa7e:	01 d2                	add    %edx,%edx
    fa80:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
    fa83:	29 c6                	sub    %eax,%esi
    fa85:	40 80 ce 30          	or     $0x30,%sil
    fa89:	40 88 74 24 0c       	mov    %sil,0xc(%rsp)
    fa8e:	48 81 f9 00 e1 f5 05 	cmp    $0x5f5e100,%rcx
    fa95:	73 0f                	jae    faa6 <pt_str_from_int+0x246>
    fa97:	b8 0c 00 00 00       	mov    $0xc,%eax
    fa9c:	b9 0b 00 00 00       	mov    $0xb,%ecx
    faa1:	e9 0f 03 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    faa6:	48 ba fd ce 61 84 11 	movabs $0xabcc77118461cefd,%rdx
    faad:	77 cc ab 
    fab0:	48 89 c8             	mov    %rcx,%rax
    fab3:	48 f7 e2             	mul    %rdx
    fab6:	48 89 d6             	mov    %rdx,%rsi
    fab9:	48 c1 ee 1a          	shr    $0x1a,%rsi
    fabd:	48 89 f0             	mov    %rsi,%rax
    fac0:	49 f7 e0             	mul    %r8
    fac3:	01 d2                	add    %edx,%edx
    fac5:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
    fac8:	29 c6                	sub    %eax,%esi
    faca:	40 80 ce 30          	or     $0x30,%sil
    face:	40 88 74 24 0b       	mov    %sil,0xb(%rsp)
    fad3:	48 81 f9 00 ca 9a 3b 	cmp    $0x3b9aca00,%rcx
    fada:	73 0f                	jae    faeb <pt_str_from_int+0x28b>
    fadc:	b8 0b 00 00 00       	mov    $0xb,%eax
    fae1:	b9 0a 00 00 00       	mov    $0xa,%ecx
    fae6:	e9 ca 02 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    faeb:	48 89 c8             	mov    %rcx,%rax
    faee:	48 c1 e8 09          	shr    $0x9,%rax
    faf2:	48 ba 53 5a 9b a0 2f 	movabs $0x44b82fa09b5a53,%rdx
    faf9:	b8 44 00 
    fafc:	48 f7 e2             	mul    %rdx
    faff:	48 89 d6             	mov    %rdx,%rsi
    fb02:	48 c1 ee 0b          	shr    $0xb,%rsi
    fb06:	48 89 f0             	mov    %rsi,%rax
    fb09:	49 f7 e0             	mul    %r8
    fb0c:	01 d2                	add    %edx,%edx
    fb0e:	8d 04 92             	lea    (%rdx,%rdx,4),%eax
    fb11:	29 c6                	sub    %eax,%esi
    fb13:	40 80 ce 30          	or     $0x30,%sil
    fb17:	40 88 74 24 0a       	mov    %sil,0xa(%rsp)
    fb1c:	48 89 c8             	mov    %rcx,%rax
    fb1f:	48 c1 e8 0a          	shr    $0xa,%rax
    fb23:	48 3d f9 02 95 00    	cmp    $0x9502f9,%rax
    fb29:	73 0f                	jae    fb3a <pt_str_from_int+0x2da>
    fb2b:	b8 0a 00 00 00       	mov    $0xa,%eax
    fb30:	b9 09 00 00 00       	mov    $0x9,%ecx
    fb35:	e9 7b 02 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    fb3a:	48 ba bf d5 ed bd ce 	movabs $0xdbe6fecebdedd5bf,%rdx
    fb41:	fe e6 db 
    fb44:	48 89 c8             	mov    %rcx,%rax
    fb47:	48 f7 e2             	mul    %rdx
    fb4a:	48 c1 ea 21          	shr    $0x21,%rdx
    fb4e:	48 69 c2 67 66 66 66 	imul   $0x66666667,%rdx,%rax
    fb55:	48 c1 e8 22          	shr    $0x22,%rax
    fb59:	01 c0                	add    %eax,%eax
    fb5b:	8d 04 80             	lea    (%rax,%rax,4),%eax
    fb5e:	29 c2                	sub    %eax,%edx
    fb60:	80 ca 30             	or     $0x30,%dl
    fb63:	88 54 24 09          	mov    %dl,0x9(%rsp)
    fb67:	48 89 c8             	mov    %rcx,%rax
    fb6a:	48 c1 e8 0b          	shr    $0xb,%rax
    fb6e:	48 3d dd 0e e9 02    	cmp    $0x2e90edd,%rax
    fb74:	73 0f                	jae    fb85 <pt_str_from_int+0x325>
    fb76:	b8 09 00 00 00       	mov    $0x9,%eax
    fb7b:	b9 08 00 00 00       	mov    $0x8,%ecx
    fb80:	e9 30 02 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    fb85:	48 ba ff aa 24 cb 0b 	movabs $0xafebff0bcb24aaff,%rdx
    fb8c:	ff eb af 
    fb8f:	48 89 c8             	mov    %rcx,%rax
    fb92:	48 f7 e2             	mul    %rdx
    fb95:	48 c1 ea 24          	shr    $0x24,%rdx
    fb99:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
    fba0:	48 c1 e8 20          	shr    $0x20,%rax
    fba4:	01 c0                	add    %eax,%eax
    fba6:	8d 04 80             	lea    (%rax,%rax,4),%eax
    fba9:	29 c2                	sub    %eax,%edx
    fbab:	80 ca 30             	or     $0x30,%dl
    fbae:	88 54 24 08          	mov    %dl,0x8(%rsp)
    fbb2:	48 89 c8             	mov    %rcx,%rax
    fbb5:	48 c1 e8 0c          	shr    $0xc,%rax
    fbb9:	48 3d 51 4a 8d 0e    	cmp    $0xe8d4a51,%rax
    fbbf:	73 0f                	jae    fbd0 <pt_str_from_int+0x370>
    fbc1:	b8 08 00 00 00       	mov    $0x8,%eax
    fbc6:	b9 07 00 00 00       	mov    $0x7,%ecx
    fbcb:	e9 e5 01 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    fbd0:	48 ba 33 22 d4 5b 02 	movabs $0x232f33025bd42233,%rdx
    fbd7:	33 2f 23 
    fbda:	48 89 c8             	mov    %rcx,%rax
    fbdd:	48 f7 e2             	mul    %rdx
    fbe0:	48 c1 ea 25          	shr    $0x25,%rdx
    fbe4:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
    fbeb:	48 c1 e8 20          	shr    $0x20,%rax
    fbef:	01 c0                	add    %eax,%eax
    fbf1:	8d 04 80             	lea    (%rax,%rax,4),%eax
    fbf4:	29 c2                	sub    %eax,%edx
    fbf6:	80 ca 30             	or     $0x30,%dl
    fbf9:	88 54 24 07          	mov    %dl,0x7(%rsp)
    fbfd:	48 89 c8             	mov    %rcx,%rax
    fc00:	48 c1 e8 0d          	shr    $0xd,%rax
    fc04:	48 3d 95 73 c2 48    	cmp    $0x48c27395,%rax
    fc0a:	73 0f                	jae    fc1b <pt_str_from_int+0x3bb>
    fc0c:	b8 07 00 00 00       	mov    $0x7,%eax
    fc11:	b9 06 00 00 00       	mov    $0x6,%ecx
    fc16:	e9 9a 01 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    fc1b:	48 ba 85 03 ed 92 d0 	movabs $0x384b84d092ed0385,%rdx
    fc22:	84 4b 38 
    fc25:	48 89 c8             	mov    %rcx,%rax
    fc28:	48 f7 e2             	mul    %rdx
    fc2b:	48 c1 ea 29          	shr    $0x29,%rdx
    fc2f:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
    fc36:	48 c1 e8 20          	shr    $0x20,%rax
    fc3a:	01 c0                	add    %eax,%eax
    fc3c:	8d 04 80             	lea    (%rax,%rax,4),%eax
    fc3f:	29 c2                	sub    %eax,%edx
    fc41:	80 ca 30             	or     $0x30,%dl
    fc44:	88 54 24 06          	mov    %dl,0x6(%rsp)
    fc48:	48 b8 00 40 7a 10 f3 	movabs $0x5af3107a4000,%rax
    fc4f:	5a 00 00 
    fc52:	48 39 c1             	cmp    %rax,%rcx
    fc55:	73 0f                	jae    fc66 <pt_str_from_int+0x406>
    fc57:	b8 06 00 00 00       	mov    $0x6,%eax
    fc5c:	b9 05 00 00 00       	mov    $0x5,%ecx
    fc61:	e9 4f 01 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    fc66:	48 ba 81 cd 95 50 c3 	movabs $0xb424dc35095cd81,%rdx
    fc6d:	4d 42 0b 
    fc70:	48 89 c8             	mov    %rcx,%rax
    fc73:	48 f7 e2             	mul    %rdx
    fc76:	48 c1 ea 2a          	shr    $0x2a,%rdx
    fc7a:	48 69 c2 9a 99 99 19 	imul   $0x1999999a,%rdx,%rax
    fc81:	48 c1 e8 20          	shr    $0x20,%rax
    fc85:	01 c0                	add    %eax,%eax
    fc87:	8d 04 80             	lea    (%rax,%rax,4),%eax
    fc8a:	29 c2                	sub    %eax,%edx
    fc8c:	80 ca 30             	or     $0x30,%dl
    fc8f:	88 54 24 05          	mov    %dl,0x5(%rsp)
    fc93:	48 b8 00 80 c6 a4 7e 	movabs $0x38d7ea4c68000,%rax
    fc9a:	8d 03 00 
    fc9d:	48 39 c1             	cmp    %rax,%rcx
    fca0:	73 0f                	jae    fcb1 <pt_str_from_int+0x451>
    fca2:	b8 05 00 00 00       	mov    $0x5,%eax
    fca7:	b9 04 00 00 00       	mov    $0x4,%ecx
    fcac:	e9 04 01 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    fcb1:	48 89 c8             	mov    %rcx,%rax
    fcb4:	48 c1 e8 0f          	shr    $0xf,%rax
    fcb8:	48 ba c3 ea dc f3 75 	movabs $0x24075f3dceac3,%rdx
    fcbf:	40 02 00 
    fcc2:	48 f7 e2             	mul    %rdx
    fcc5:	48 c1 ea 14          	shr    $0x14,%rdx
    fcc9:	69 c2 67 66 00 00    	imul   $0x6667,%edx,%eax
    fccf:	c1 e8 11             	shr    $0x11,%eax
    fcd2:	83 e0 fe             	and    $0xfffffffe,%eax
    fcd5:	8d 04 80             	lea    (%rax,%rax,4),%eax
    fcd8:	29 c2                	sub    %eax,%edx
    fcda:	80 ca 30             	or     $0x30,%dl
    fcdd:	88 54 24 04          	mov    %dl,0x4(%rsp)
    fce1:	48 b8 00 00 c1 6f f2 	movabs $0x2386f26fc10000,%rax
    fce8:	86 23 00 
    fceb:	48 39 c1             	cmp    %rax,%rcx
    fcee:	73 0f                	jae    fcff <pt_str_from_int+0x49f>
    fcf0:	b8 04 00 00 00       	mov    $0x4,%eax
    fcf5:	b9 03 00 00 00       	mov    $0x3,%ecx
    fcfa:	e9 b6 00 00 00       	jmp    fdb5 <pt_str_from_int+0x555>
    fcff:	48 ba 57 78 13 b1 2f 	movabs $0x39a5652fb1137857,%rdx
    fd06:	65 a5 39 
    fd09:	48 89 c8             	mov    %rcx,%rax
    fd0c:	48 f7 e2             	mul    %rdx
    fd0f:	48 c1 ea 33          	shr    $0x33,%rdx
    fd13:	69 c2 9a 19 00 00    	imul   $0x199a,%edx,%eax
    fd19:	c1 e8 0f             	shr    $0xf,%eax
    fd1c:	83 e0 fe             	and    $0xfffffffe,%eax
    fd1f:	8d 04 80             	lea    (%rax,%rax,4),%eax
    fd22:	29 c2                	sub    %eax,%edx
    fd24:	80 ca 30             	or     $0x30,%dl
    fd27:	88 54 24 03          	mov    %dl,0x3(%rsp)
    fd2b:	48 b8 00 00 8a 5d 78 	movabs $0x16345785d8a0000,%rax
    fd32:	45 63 01 
    fd35:	48 39 c1             	cmp    %rax,%rcx
    fd38:	73 0c                	jae    fd46 <pt_str_from_int+0x4e6>
    fd3a:	b8 03 00 00 00       	mov    $0x3,%eax
    fd3f:	b9 02 00 00 00       	mov    $0x2,%ecx
    fd44:	eb 6f                	jmp    fdb5 <pt_str_from_int+0x555>
    fd46:	48 89 c8             	mov    %rcx,%rax
    fd49:	48 c1 e8 11          	shr    $0x11,%rax
    fd4d:	48 ba 53 1b 19 d5 3b 	movabs $0x5c3bd5191b53,%rdx
    fd54:	5c 00 00 
    fd57:	48 f7 e2             	mul    %rdx
    fd5a:	c1 ea 16             	shr    $0x16,%edx
    fd5d:	69 c2 cd 00 00 00    	imul   $0xcd,%edx,%eax
    fd63:	c1 e8 0a             	shr    $0xa,%eax
    fd66:	83 e0 fe             	and    $0xfffffffe,%eax
    fd69:	8d 04 80             	lea    (%rax,%rax,4),%eax
    fd6c:	28 c2                	sub    %al,%dl
    fd6e:	80 ca 30             	or     $0x30,%dl
    fd71:	88 54 24 02          	mov    %dl,0x2(%rsp)
    fd75:	48 b8 00 00 64 a7 b3 	movabs $0xde0b6b3a7640000,%rax
    fd7c:	b6 e0 0d 
    fd7f:	48 39 c1             	cmp    %rax,%rcx
    fd82:	73 0c                	jae    fd90 <pt_str_from_int+0x530>
    fd84:	b8 02 00 00 00       	mov    $0x2,%eax
    fd89:	b9 01 00 00 00       	mov    $0x1,%ecx
    fd8e:	eb 25                	jmp    fdb5 <pt_str_from_int+0x555>
    fd90:	48 c1 e9 12          	shr    $0x12,%rcx
    fd94:	48 ba 0f 49 47 77 c9 	movabs $0x49c97747490f,%rdx
    fd9b:	49 00 00 
    fd9e:	48 89 c8             	mov    %rcx,%rax
    fda1:	48 f7 e2             	mul    %rdx
    fda4:	c1 ea 18             	shr    $0x18,%edx
    fda7:	80 ca 30             	or     $0x30,%dl
    fdaa:	88 54 24 01          	mov    %dl,0x1(%rsp)
    fdae:	b8 01 00 00 00       	mov    $0x1,%eax
    fdb3:	31 c9                	xor    %ecx,%ecx
    fdb5:	48 85 ff             	test   %rdi,%rdi
    fdb8:	79 07                	jns    fdc1 <pt_str_from_int+0x561>
    fdba:	c6 04 0c 2d          	movb   $0x2d,(%rsp,%rcx,1)
    fdbe:	48 89 c8             	mov    %rcx,%rax
    fdc1:	ba 14 00 00 00       	mov    $0x14,%edx
    fdc6:	48 29 c2             	sub    %rax,%rdx
    fdc9:	48 8d 34 04          	lea    (%rsp,%rax,1),%rsi
    fdcd:	48 8d 7c 24 20       	lea    0x20(%rsp),%rdi
    fdd2:	ff 15 00 3c 05 00    	call   *0x53c00(%rip)        # 639d8 <_DYNAMIC+0x478>
    fdd8:	80 7c 24 20 00       	cmpb   $0x0,0x20(%rsp)
    fddd:	0f 85 98 00 00 00    	jne    fe7b <pt_str_from_int+0x61b>
    fde3:	48 8b 5c 24 30       	mov    0x30(%rsp),%rbx
    fde8:	48 85 db             	test   %rbx,%rbx
    fdeb:	79 14                	jns    fe01 <pt_str_from_int+0x5a1>
    fded:	45 31 e4             	xor    %r12d,%r12d
    fdf0:	4c 89 e7             	mov    %r12,%rdi
    fdf3:	48 89 de             	mov    %rbx,%rsi
    fdf6:	ff 15 e4 3d 05 00    	call   *0x53de4(%rip)        # 63be0 <_DYNAMIC+0x680>
    fdfc:	e9 a5 00 00 00       	jmp    fea6 <pt_str_from_int+0x646>
    fe01:	74 38                	je     fe3b <pt_str_from_int+0x5db>
    fe03:	4c 8b 7c 24 28       	mov    0x28(%rsp),%r15
    fe08:	ff 15 fa 3b 05 00    	call   *0x53bfa(%rip)        # 63a08 <_DYNAMIC+0x4a8>
    fe0e:	41 bc 01 00 00 00    	mov    $0x1,%r12d
    fe14:	be 01 00 00 00       	mov    $0x1,%esi
    fe19:	48 89 df             	mov    %rbx,%rdi
    fe1c:	ff 15 26 3c 05 00    	call   *0x53c26(%rip)        # 63a48 <_DYNAMIC+0x4e8>
    fe22:	48 85 c0             	test   %rax,%rax
    fe25:	74 c9                	je     fdf0 <pt_str_from_int+0x590>
    fe27:	49 89 c6             	mov    %rax,%r14
    fe2a:	48 89 c7             	mov    %rax,%rdi
    fe2d:	4c 89 fe             	mov    %r15,%rsi
    fe30:	48 89 da             	mov    %rbx,%rdx
    fe33:	ff 15 4f 41 05 00    	call   *0x5414f(%rip)        # 63f88 <memcpy@GLIBC_2.14>
    fe39:	eb 06                	jmp    fe41 <pt_str_from_int+0x5e1>
    fe3b:	41 be 01 00 00 00    	mov    $0x1,%r14d
    fe41:	ff 15 c1 3b 05 00    	call   *0x53bc1(%rip)        # 63a08 <_DYNAMIC+0x4a8>
    fe47:	bf 20 00 00 00       	mov    $0x20,%edi
    fe4c:	be 08 00 00 00       	mov    $0x8,%esi
    fe51:	ff 15 f1 3b 05 00    	call   *0x53bf1(%rip)        # 63a48 <_DYNAMIC+0x4e8>
    fe57:	48 85 c0             	test   %rax,%rax
    fe5a:	74 3a                	je     fe96 <pt_str_from_int+0x636>
    fe5c:	4c 89 30             	mov    %r14,(%rax)
    fe5f:	48 89 58 08          	mov    %rbx,0x8(%rax)
    fe63:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
    fe6a:	00 
    fe6b:	48 89 58 18          	mov    %rbx,0x18(%rax)
    fe6f:	48 83 c4 38          	add    $0x38,%rsp
    fe73:	5b                   	pop    %rbx
    fe74:	41 5c                	pop    %r12
    fe76:	41 5e                	pop    %r14
    fe78:	41 5f                	pop    %r15
    fe7a:	c3                   	ret
    fe7b:	48 8d 3d 4e 2e 04 00 	lea    0x42e4e(%rip),%rdi        # 52cd0 <anon.13df560d6b83da2c10641ad9b2156d4a.31.llvm.366253514071984098+0x465>
    fe82:	48 8d 15 4f 15 05 00 	lea    0x5154f(%rip),%rdx        # 613d8 <__do_global_dtors_aux_fini_array_entry+0x110>
    fe89:	be 97 00 00 00       	mov    $0x97,%esi
    fe8e:	ff 15 74 3c 05 00    	call   *0x53c74(%rip)        # 63b08 <_DYNAMIC+0x5a8>
    fe94:	eb 10                	jmp    fea6 <pt_str_from_int+0x646>
    fe96:	bf 08 00 00 00       	mov    $0x8,%edi
    fe9b:	be 20 00 00 00       	mov    $0x20,%esi
    fea0:	ff 15 aa 3d 05 00    	call   *0x53daa(%rip)        # 63c50 <_DYNAMIC+0x6f0>
    fea6:	0f 0b                	ud2
    fea8:	48 85 db             	test   %rbx,%rbx
    feab:	74 17                	je     fec4 <pt_str_from_int+0x664>
    fead:	ba 01 00 00 00       	mov    $0x1,%edx
    feb2:	4c 89 f7             	mov    %r14,%rdi
    feb5:	48 89 de             	mov    %rbx,%rsi
    feb8:	ff 15 6a 40 05 00    	call   *0x5406a(%rip)        # 63f28 <_DYNAMIC+0x9c8>
    febe:	ff 15 c4 39 05 00    	call   *0x539c4(%rip)        # 63888 <_DYNAMIC+0x328>
    fec4:	ff 15 be 39 05 00    	call   *0x539be(%rip)        # 63888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card817/baseline/strings:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

0000000000010160 <pt_str_new>:
   10160:	41 57                	push   %r15
   10162:	41 56                	push   %r14
   10164:	53                   	push   %rbx
   10165:	48 83 ec 20          	sub    $0x20,%rsp
   10169:	48 89 f2             	mov    %rsi,%rdx
   1016c:	48 89 fe             	mov    %rdi,%rsi
   1016f:	48 8d 7c 24 08       	lea    0x8(%rsp),%rdi
   10174:	ff 15 56 3d 05 00    	call   *0x53d56(%rip)        # 63ed0 <_DYNAMIC+0x970>
   1017a:	48 83 7c 24 08 ff    	cmpq   $0xffffffffffffffff,0x8(%rsp)
   10180:	74 25                	je     101a7 <pt_str_new+0x47>
   10182:	48 8d 7c 24 08       	lea    0x8(%rsp),%rdi
   10187:	ff 15 63 3c 05 00    	call   *0x53c63(%rip)        # 63df0 <_DYNAMIC+0x890>
   1018d:	49 89 c6             	mov    %rax,%r14
   10190:	48 89 d3             	mov    %rdx,%rbx
   10193:	48 83 fb 20          	cmp    $0x20,%rbx
   10197:	73 64                	jae    101fd <pt_str_new+0x9d>
   10199:	4c 89 f7             	mov    %r14,%rdi
   1019c:	48 89 de             	mov    %rbx,%rsi
   1019f:	ff 15 5b 3b 05 00    	call   *0x53b5b(%rip)        # 63d00 <_DYNAMIC+0x7a0>
   101a5:	eb 62                	jmp    10209 <pt_str_new+0xa9>
   101a7:	4c 8b 7c 24 10       	mov    0x10(%rsp),%r15
   101ac:	48 8b 5c 24 18       	mov    0x18(%rsp),%rbx
   101b1:	48 85 db             	test   %rbx,%rbx
   101b4:	74 2c                	je     101e2 <pt_str_new+0x82>
   101b6:	ff 15 4c 38 05 00    	call   *0x5384c(%rip)        # 63a08 <_DYNAMIC+0x4a8>
   101bc:	be 01 00 00 00       	mov    $0x1,%esi
   101c1:	48 89 df             	mov    %rbx,%rdi
   101c4:	ff 15 7e 38 05 00    	call   *0x5387e(%rip)        # 63a48 <_DYNAMIC+0x4e8>
   101ca:	49 89 c6             	mov    %rax,%r14
   101cd:	48 85 c0             	test   %rax,%rax
   101d0:	75 16                	jne    101e8 <pt_str_new+0x88>
   101d2:	bf 01 00 00 00       	mov    $0x1,%edi
   101d7:	48 89 de             	mov    %rbx,%rsi
   101da:	ff 15 70 3a 05 00    	call   *0x53a70(%rip)        # 63c50 <_DYNAMIC+0x6f0>
   101e0:	eb 72                	jmp    10254 <pt_str_new+0xf4>
   101e2:	41 be 01 00 00 00    	mov    $0x1,%r14d
   101e8:	4c 89 f7             	mov    %r14,%rdi
   101eb:	4c 89 fe             	mov    %r15,%rsi
   101ee:	48 89 da             	mov    %rbx,%rdx
   101f1:	ff 15 91 3d 05 00    	call   *0x53d91(%rip)        # 63f88 <memcpy@GLIBC_2.14>
   101f7:	48 83 fb 20          	cmp    $0x20,%rbx
   101fb:	72 9c                	jb     10199 <pt_str_new+0x39>
   101fd:	4c 89 f7             	mov    %r14,%rdi
   10200:	48 89 de             	mov    %rbx,%rsi
   10203:	ff 15 b7 3c 05 00    	call   *0x53cb7(%rip)        # 63ec0 <_DYNAMIC+0x960>
   10209:	49 89 c7             	mov    %rax,%r15
   1020c:	ff 15 f6 37 05 00    	call   *0x537f6(%rip)        # 63a08 <_DYNAMIC+0x4a8>
   10212:	bf 20 00 00 00       	mov    $0x20,%edi
   10217:	be 08 00 00 00       	mov    $0x8,%esi
   1021c:	ff 15 26 38 05 00    	call   *0x53826(%rip)        # 63a48 <_DYNAMIC+0x4e8>
   10222:	48 85 c0             	test   %rax,%rax
   10225:	74 1d                	je     10244 <pt_str_new+0xe4>
   10227:	4c 89 30             	mov    %r14,(%rax)
   1022a:	48 89 58 08          	mov    %rbx,0x8(%rax)
   1022e:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
   10235:	00 
   10236:	4c 89 78 18          	mov    %r15,0x18(%rax)
   1023a:	48 83 c4 20          	add    $0x20,%rsp
   1023e:	5b                   	pop    %rbx
   1023f:	41 5e                	pop    %r14
   10241:	41 5f                	pop    %r15
   10243:	c3                   	ret
   10244:	bf 08 00 00 00       	mov    $0x8,%edi
   10249:	be 20 00 00 00       	mov    $0x20,%esi
   1024e:	ff 15 fc 39 05 00    	call   *0x539fc(%rip)        # 63c50 <_DYNAMIC+0x6f0>
   10254:	0f 0b                	ud2
   10256:	eb 19                	jmp    10271 <pt_str_new+0x111>
   10258:	eb 0c                	jmp    10266 <pt_str_new+0x106>
   1025a:	ff 15 28 36 05 00    	call   *0x53628(%rip)        # 63888 <_DYNAMIC+0x328>
   10260:	ff 15 22 36 05 00    	call   *0x53622(%rip)        # 63888 <_DYNAMIC+0x328>
   10266:	48 85 db             	test   %rbx,%rbx
   10269:	75 06                	jne    10271 <pt_str_new+0x111>
   1026b:	ff 15 17 36 05 00    	call   *0x53617(%rip)        # 63888 <_DYNAMIC+0x328>
   10271:	ba 01 00 00 00       	mov    $0x1,%edx
   10276:	4c 89 f7             	mov    %r14,%rdi
   10279:	48 89 de             	mov    %rbx,%rsi
   1027c:	ff 15 a6 3c 05 00    	call   *0x53ca6(%rip)        # 63f28 <_DYNAMIC+0x9c8>
   10282:	ff 15 00 36 05 00    	call   *0x53600(%rip)        # 63888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card817/baseline/strings:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000fed0 <pt_str_index>:
    fed0:	41 57                	push   %r15
    fed2:	41 56                	push   %r14
    fed4:	41 54                	push   %r12
    fed6:	53                   	push   %rbx
    fed7:	50                   	push   %rax
    fed8:	48 8b 47 18          	mov    0x18(%rdi),%rax
    fedc:	48 85 c0             	test   %rax,%rax
    fedf:	0f 88 21 01 00 00    	js     10006 <pt_str_index+0x136>
    fee5:	49 89 f4             	mov    %rsi,%r12
    fee8:	49 c1 fc 3f          	sar    $0x3f,%r12
    feec:	49 21 c4             	and    %rax,%r12
    feef:	49 01 f4             	add    %rsi,%r12
    fef2:	0f 99 c1             	setns  %cl
    fef5:	49 39 c4             	cmp    %rax,%r12
    fef8:	0f 92 c2             	setb   %dl
    fefb:	20 ca                	and    %cl,%dl
    fefd:	80 fa 01             	cmp    $0x1,%dl
    ff00:	0f 85 00 01 00 00    	jne    10006 <pt_str_index+0x136>
    ff06:	48 8b 1f             	mov    (%rdi),%rbx
    ff09:	48 8b 77 08          	mov    0x8(%rdi),%rsi
    ff0d:	48 39 f0             	cmp    %rsi,%rax
    ff10:	75 33                	jne    ff45 <pt_str_index+0x75>
    ff12:	4d 85 e4             	test   %r12,%r12
    ff15:	74 0b                	je     ff22 <pt_str_index+0x52>
    ff17:	42 80 3c 23 bf       	cmpb   $0xbf,(%rbx,%r12,1)
    ff1c:	0f 8e e4 00 00 00    	jle    10006 <pt_str_index+0x136>
    ff22:	49 8d 4c 24 01       	lea    0x1(%r12),%rcx
    ff27:	41 be 01 00 00 00    	mov    $0x1,%r14d
    ff2d:	48 39 c1             	cmp    %rax,%rcx
    ff30:	0f 83 ca 00 00 00    	jae    10000 <pt_str_index+0x130>
    ff36:	80 3c 0b bf          	cmpb   $0xbf,(%rbx,%rcx,1)
    ff3a:	0f 8f 4a 01 00 00    	jg     1008a <pt_str_index+0x1ba>
    ff40:	e9 c1 00 00 00       	jmp    10006 <pt_str_index+0x136>
    ff45:	48 8d 0c 33          	lea    (%rbx,%rsi,1),%rcx
    ff49:	4d 85 e4             	test   %r12,%r12
    ff4c:	74 56                	je     ffa4 <pt_str_index+0xd4>
    ff4e:	31 c0                	xor    %eax,%eax
    ff50:	31 d2                	xor    %edx,%edx
    ff52:	49 89 d8             	mov    %rbx,%r8
    ff55:	eb 1e                	jmp    ff75 <pt_str_index+0xa5>
    ff57:	66 0f 1f 84 00 00 00 	nopw   0x0(%rax,%rax,1)
    ff5e:	00 00 
    ff60:	49 8d 78 01          	lea    0x1(%r8),%rdi
    ff64:	49 89 f9             	mov    %rdi,%r9
    ff67:	4d 29 c1             	sub    %r8,%r9
    ff6a:	4c 01 ca             	add    %r9,%rdx
    ff6d:	49 89 f8             	mov    %rdi,%r8
    ff70:	49 ff cc             	dec    %r12
    ff73:	74 34                	je     ffa9 <pt_str_index+0xd9>
    ff75:	49 39 c8             	cmp    %rcx,%r8
    ff78:	0f 84 8a 00 00 00    	je     10008 <pt_str_index+0x138>
    ff7e:	41 0f b6 38          	movzbl (%r8),%edi
    ff82:	40 84 ff             	test   %dil,%dil
    ff85:	79 d9                	jns    ff60 <pt_str_index+0x90>
    ff87:	40 80 ff e0          	cmp    $0xe0,%dil
    ff8b:	72 11                	jb     ff9e <pt_str_index+0xce>
    ff8d:	40 80 ff f0          	cmp    $0xf0,%dil
    ff91:	4c 89 c7             	mov    %r8,%rdi
    ff94:	48 83 df ff          	sbb    $0xffffffffffffffff,%rdi
    ff98:	48 83 c7 03          	add    $0x3,%rdi
    ff9c:	eb c6                	jmp    ff64 <pt_str_index+0x94>
    ff9e:	49 8d 78 02          	lea    0x2(%r8),%rdi
    ffa2:	eb c0                	jmp    ff64 <pt_str_index+0x94>
    ffa4:	31 d2                	xor    %edx,%edx
    ffa6:	48 89 df             	mov    %rbx,%rdi
    ffa9:	48 39 cf             	cmp    %rcx,%rdi
    ffac:	74 58                	je     10006 <pt_str_index+0x136>
    ffae:	44 0f b6 07          	movzbl (%rdi),%r8d
    ffb2:	41 be 01 00 00 00    	mov    $0x1,%r14d
    ffb8:	45 84 c0             	test   %r8b,%r8b
    ffbb:	0f 89 83 00 00 00    	jns    10044 <pt_str_index+0x174>
    ffc1:	44 89 c0             	mov    %r8d,%eax
    ffc4:	83 e0 1f             	and    $0x1f,%eax
    ffc7:	44 0f b6 4f 01       	movzbl 0x1(%rdi),%r9d
    ffcc:	41 83 e1 3f          	and    $0x3f,%r9d
    ffd0:	41 80 f8 e0          	cmp    $0xe0,%r8b
    ffd4:	72 3e                	jb     10014 <pt_str_index+0x144>
    ffd6:	0f b6 4f 02          	movzbl 0x2(%rdi),%ecx
    ffda:	41 c1 e1 06          	shl    $0x6,%r9d
    ffde:	83 e1 3f             	and    $0x3f,%ecx
    ffe1:	44 09 c9             	or     %r9d,%ecx
    ffe4:	41 80 f8 f0          	cmp    $0xf0,%r8b
    ffe8:	72 32                	jb     1001c <pt_str_index+0x14c>
    ffea:	0f b6 7f 03          	movzbl 0x3(%rdi),%edi
    ffee:	83 e0 07             	and    $0x7,%eax
    fff1:	c1 e0 12             	shl    $0x12,%eax
    fff4:	c1 e1 06             	shl    $0x6,%ecx
    fff7:	83 e7 3f             	and    $0x3f,%edi
    fffa:	09 cf                	or     %ecx,%edi
    fffc:	09 f8                	or     %edi,%eax
    fffe:	eb 21                	jmp    10021 <pt_str_index+0x151>
   10000:	0f 84 84 00 00 00    	je     1008a <pt_str_index+0x1ba>
   10006:	31 c0                	xor    %eax,%eax
   10008:	48 83 c4 08          	add    $0x8,%rsp
   1000c:	5b                   	pop    %rbx
   1000d:	41 5c                	pop    %r12
   1000f:	41 5e                	pop    %r14
   10011:	41 5f                	pop    %r15
   10013:	c3                   	ret
   10014:	c1 e0 06             	shl    $0x6,%eax
   10017:	44 09 c8             	or     %r9d,%eax
   1001a:	eb 05                	jmp    10021 <pt_str_index+0x151>
   1001c:	c1 e0 0c             	shl    $0xc,%eax
   1001f:	09 c8                	or     %ecx,%eax
   10021:	3d 80 00 00 00       	cmp    $0x80,%eax
   10026:	72 1c                	jb     10044 <pt_str_index+0x174>
   10028:	41 be 02 00 00 00    	mov    $0x2,%r14d
   1002e:	3d 00 08 00 00       	cmp    $0x800,%eax
   10033:	72 0f                	jb     10044 <pt_str_index+0x174>
   10035:	3d 00 00 01 00       	cmp    $0x10000,%eax
   1003a:	41 be 04 00 00 00    	mov    $0x4,%r14d
   10040:	49 83 de 00          	sbb    $0x0,%r14
   10044:	4c 89 f1             	mov    %r14,%rcx
   10047:	48 01 d1             	add    %rdx,%rcx
   1004a:	72 24                	jb     10070 <pt_str_index+0x1a0>
   1004c:	48 85 d2             	test   %rdx,%rdx
   1004f:	74 0f                	je     10060 <pt_str_index+0x190>
   10051:	48 39 f2             	cmp    %rsi,%rdx
   10054:	73 08                	jae    1005e <pt_str_index+0x18e>
   10056:	80 3c 13 bf          	cmpb   $0xbf,(%rbx,%rdx,1)
   1005a:	7f 04                	jg     10060 <pt_str_index+0x190>
   1005c:	eb 12                	jmp    10070 <pt_str_index+0x1a0>
   1005e:	75 10                	jne    10070 <pt_str_index+0x1a0>
   10060:	48 85 c9             	test   %rcx,%rcx
   10063:	74 22                	je     10087 <pt_str_index+0x1b7>
   10065:	48 39 f1             	cmp    %rsi,%rcx
   10068:	73 1b                	jae    10085 <pt_str_index+0x1b5>
   1006a:	80 3c 0b bf          	cmpb   $0xbf,(%rbx,%rcx,1)
   1006e:	7f 17                	jg     10087 <pt_str_index+0x1b7>
   10070:	4c 8d 05 49 13 05 00 	lea    0x51349(%rip),%r8        # 613c0 <__do_global_dtors_aux_fini_array_entry+0xf8>
   10077:	48 89 df             	mov    %rbx,%rdi
   1007a:	ff 15 20 37 05 00    	call   *0x53720(%rip)        # 637a0 <_DYNAMIC+0x240>
   10080:	e9 95 00 00 00       	jmp    1011a <pt_str_index+0x24a>
   10085:	75 e9                	jne    10070 <pt_str_index+0x1a0>
   10087:	49 89 d4             	mov    %rdx,%r12
   1008a:	ff 15 78 39 05 00    	call   *0x53978(%rip)        # 63a08 <_DYNAMIC+0x4a8>
   10090:	be 01 00 00 00       	mov    $0x1,%esi
   10095:	4c 89 f7             	mov    %r14,%rdi
   10098:	ff 15 aa 39 05 00    	call   *0x539aa(%rip)        # 63a48 <_DYNAMIC+0x4e8>
   1009e:	48 85 c0             	test   %rax,%rax
   100a1:	74 69                	je     1010c <pt_str_index+0x23c>
   100a3:	49 89 c7             	mov    %rax,%r15
   100a6:	4c 01 e3             	add    %r12,%rbx
   100a9:	48 89 c7             	mov    %rax,%rdi
   100ac:	48 89 de             	mov    %rbx,%rsi
   100af:	4c 89 f2             	mov    %r14,%rdx
   100b2:	ff 15 d0 3e 05 00    	call   *0x53ed0(%rip)        # 63f88 <memcpy@GLIBC_2.14>
   100b8:	4c 89 ff             	mov    %r15,%rdi
   100bb:	4c 89 f6             	mov    %r14,%rsi
   100be:	ff 15 3c 3c 05 00    	call   *0x53c3c(%rip)        # 63d00 <_DYNAMIC+0x7a0>
   100c4:	48 89 c3             	mov    %rax,%rbx
   100c7:	ff 15 3b 39 05 00    	call   *0x5393b(%rip)        # 63a08 <_DYNAMIC+0x4a8>
   100cd:	bf 20 00 00 00       	mov    $0x20,%edi
   100d2:	be 08 00 00 00       	mov    $0x8,%esi
   100d7:	ff 15 6b 39 05 00    	call   *0x5396b(%rip)        # 63a48 <_DYNAMIC+0x4e8>
   100dd:	48 85 c0             	test   %rax,%rax
   100e0:	74 18                	je     100fa <pt_str_index+0x22a>
   100e2:	4c 89 38             	mov    %r15,(%rax)
   100e5:	4c 89 70 08          	mov    %r14,0x8(%rax)
   100e9:	48 c7 40 10 01 00 00 	movq   $0x1,0x10(%rax)
   100f0:	00 
   100f1:	48 89 58 18          	mov    %rbx,0x18(%rax)
   100f5:	e9 0e ff ff ff       	jmp    10008 <pt_str_index+0x138>
   100fa:	bf 08 00 00 00       	mov    $0x8,%edi
   100ff:	be 20 00 00 00       	mov    $0x20,%esi
   10104:	ff 15 46 3b 05 00    	call   *0x53b46(%rip)        # 63c50 <_DYNAMIC+0x6f0>
   1010a:	eb 0e                	jmp    1011a <pt_str_index+0x24a>
   1010c:	bf 01 00 00 00       	mov    $0x1,%edi
   10111:	4c 89 f6             	mov    %r14,%rsi
   10114:	ff 15 36 3b 05 00    	call   *0x53b36(%rip)        # 63c50 <_DYNAMIC+0x6f0>
   1011a:	0f 0b                	ud2
   1011c:	eb 06                	jmp    10124 <pt_str_index+0x254>
   1011e:	ff 15 64 37 05 00    	call   *0x53764(%rip)        # 63888 <_DYNAMIC+0x328>
   10124:	ba 01 00 00 00       	mov    $0x1,%edx
   10129:	4c 89 ff             	mov    %r15,%rdi
   1012c:	4c 89 f6             	mov    %r14,%rsi
   1012f:	ff 15 f3 3d 05 00    	call   *0x53df3(%rip)        # 63f28 <_DYNAMIC+0x9c8>
   10135:	ff 15 4d 37 05 00    	call   *0x5374d(%rip)        # 63888 <_DYNAMIC+0x328>

Disassembly of section .fini:
