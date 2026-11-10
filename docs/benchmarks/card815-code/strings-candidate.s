
/workspace/card815-revised/candidate/strings:     file format elf64-x86-64


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
    baa3:	48 8d 3d 12 37 04 00 	lea    0x43712(%rip),%rdi        # 4f1bc <.Ldata9>
    baaa:	be 98 00 00 00       	mov    $0x98,%esi
    baaf:	48 8d 05 9a 6c 00 00 	lea    0x6c9a(%rip),%rax        # 12750 <pt_fail>
    bab6:	ff d0                	call   *%rax
    bab8:	0f 0b                	ud2
    baba:	b8 01 00 00 00       	mov    $0x1,%eax
    babf:	48 8d 35 25 38 04 00 	lea    0x43825(%rip),%rsi        # 4f2eb <.Ldatab>
    bac6:	ba 19 00 00 00       	mov    $0x19,%edx
    bacb:	48 8d 0d 32 38 04 00 	lea    0x43832(%rip),%rcx        # 4f304 <.Ldatac>
    bad2:	41 b8 43 00 00 00    	mov    $0x43,%r8d
    bad8:	4c 8d 0d 68 38 04 00 	lea    0x43868(%rip),%r9        # 4f347 <.Ldatad>
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
    bb25:	48 8d 3d 95 35 04 00 	lea    0x43595(%rip),%rdi        # 4f0c1 <.Ldata6>
    bb2c:	be 92 00 00 00       	mov    $0x92,%esi
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
