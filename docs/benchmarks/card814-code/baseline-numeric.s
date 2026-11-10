
/workspace/scratch/2ce9dd46a989/card814-base/numeric:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000aa00 <py1_steps>:
    aa00:	55                   	push   %rbp
    aa01:	48 89 e5             	mov    %rsp,%rbp
    aa04:	48 33 c0             	xor    %rax,%rax
    aa07:	49 89 c2             	mov    %rax,%r10
    aa0a:	48 83 fe 01          	cmp    $0x1,%rsi
    aa0e:	0f 85 08 00 00 00    	jne    aa1c <py1_steps+0x1c>
    aa14:	4c 89 d0             	mov    %r10,%rax
    aa17:	48 89 ec             	mov    %rbp,%rsp
    aa1a:	5d                   	pop    %rbp
    aa1b:	c3                   	ret
    aa1c:	48 89 f0             	mov    %rsi,%rax
    aa1f:	48 c1 e8 3f          	shr    $0x3f,%rax
    aa23:	48 8d 0c 06          	lea    (%rsi,%rax,1),%rcx
    aa27:	48 89 c8             	mov    %rcx,%rax
    aa2a:	48 83 e0 fe          	and    $0xfffffffffffffffe,%rax
    aa2e:	48 89 f2             	mov    %rsi,%rdx
    aa31:	48 2b d0             	sub    %rax,%rdx
    aa34:	48 3b f0             	cmp    %rax,%rsi
    aa37:	0f 95 c0             	setne  %al
    aa3a:	49 89 d1             	mov    %rdx,%r9
    aa3d:	49 83 f1 02          	xor    $0x2,%r9
    aa41:	4d 33 c0             	xor    %r8,%r8
    aa44:	49 c1 e9 3f          	shr    $0x3f,%r9
    aa48:	48 89 c7             	mov    %rax,%rdi
    aa4b:	41 23 f9             	and    %r9d,%edi
    aa4e:	41 84 c1             	test   %al,%r9b
    aa51:	4c 0f 45 05 a7 00 00 	cmovne 0xa7(%rip),%r8        # ab00 <py1_steps+0x100>
    aa58:	00 
    aa59:	4a 8d 04 02          	lea    (%rdx,%r8,1),%rax
    aa5d:	48 85 c0             	test   %rax,%rax
    aa60:	0f 84 28 00 00 00    	je     aa8e <py1_steps+0x8e>
    aa66:	b8 03 00 00 00       	mov    $0x3,%eax
    aa6b:	48 f7 ee             	imul   %rsi
    aa6e:	0f 80 58 00 00 00    	jo     aacc <py1_steps+0xcc>
    aa74:	48 83 c0 01          	add    $0x1,%rax
    aa78:	0f 90 c1             	seto   %cl
    aa7b:	84 c9                	test   %cl,%cl
    aa7d:	0f 85 32 00 00 00    	jne    aab5 <py1_steps+0xb5>
    aa83:	48 89 c6             	mov    %rax,%rsi
    aa86:	4c 89 d0             	mov    %r10,%rax
    aa89:	e9 10 00 00 00       	jmp    aa9e <py1_steps+0x9e>
    aa8e:	48 d1 f9             	sar    $1,%rcx
    aa91:	48 0f b6 c7          	movzbq %dil,%rax
    aa95:	48 2b c8             	sub    %rax,%rcx
    aa98:	4c 89 d0             	mov    %r10,%rax
    aa9b:	48 89 ce             	mov    %rcx,%rsi
    aa9e:	48 83 c0 01          	add    $0x1,%rax
    aaa2:	0f 90 c1             	seto   %cl
    aaa5:	84 c9                	test   %cl,%cl
    aaa7:	0f 85 36 00 00 00    	jne    aae3 <py1_steps+0xe3>
    aaad:	49 89 c2             	mov    %rax,%r10
    aab0:	e9 55 ff ff ff       	jmp    aa0a <py1_steps+0xa>
    aab5:	48 8d 3d b8 5a 03 00 	lea    0x35ab8(%rip),%rdi        # 40574 <.Ldatac>
    aabc:	be a6 00 00 00       	mov    $0xa6,%esi
    aac1:	48 8d 05 b8 06 00 00 	lea    0x6b8(%rip),%rax        # b180 <pt_fail>
    aac8:	ff d0                	call   *%rax
    aaca:	0f 0b                	ud2
    aacc:	48 8d 3d ff 59 03 00 	lea    0x359ff(%rip),%rdi        # 404d2 <.Ldatab>
    aad3:	be a2 00 00 00       	mov    $0xa2,%esi
    aad8:	48 8d 05 a1 06 00 00 	lea    0x6a1(%rip),%rax        # b180 <pt_fail>
    aadf:	ff d0                	call   *%rax
    aae1:	0f 0b                	ud2
    aae3:	48 8d 3d 30 58 03 00 	lea    0x35830(%rip),%rdi        # 4031a <.Ldata7>
    aaea:	be 97 00 00 00       	mov    $0x97,%esi
    aaef:	48 8d 0d 8a 06 00 00 	lea    0x68a(%rip),%rcx        # b180 <pt_fail>
    aaf6:	ff d1                	call   *%rcx
    aaf8:	0f 0b                	ud2
    aafa:	00 00                	add    %al,(%rax)
    aafc:	00 00                	add    %al,(%rax)
    aafe:	00 00                	add    %al,(%rax)
    ab00:	02 00                	add    (%rax),%al
    ab02:	00 00                	add    %al,(%rax)
    ab04:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:

/workspace/scratch/2ce9dd46a989/card814-base/numeric:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000a8a0 <pythrust_main>:
    a8a0:	55                   	push   %rbp
    a8a1:	48 89 e5             	mov    %rsp,%rbp
    a8a4:	48 83 ec 30          	sub    $0x30,%rsp
    a8a8:	48 89 1c 24          	mov    %rbx,(%rsp)
    a8ac:	4c 89 64 24 08       	mov    %r12,0x8(%rsp)
    a8b1:	4c 89 6c 24 10       	mov    %r13,0x10(%rsp)
    a8b6:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    a8bb:	4c 89 7c 24 20       	mov    %r15,0x20(%rsp)
    a8c0:	48 89 f8             	mov    %rdi,%rax
    a8c3:	41 bc 01 00 00 00    	mov    $0x1,%r12d
    a8c9:	48 33 ff             	xor    %rdi,%rdi
    a8cc:	49 89 ff             	mov    %rdi,%r15
    a8cf:	4d 89 fe             	mov    %r15,%r14
    a8d2:	49 81 fc 40 42 0f 00 	cmp    $0xf4240,%r12
    a8d9:	0f 8c 08 00 00 00    	jl     a8e7 <pythrust_main+0x47>
    a8df:	4c 89 ff             	mov    %r15,%rdi
    a8e2:	e9 77 00 00 00       	jmp    a95e <pythrust_main+0xbe>
    a8e7:	48 89 c3             	mov    %rax,%rbx
    a8ea:	4c 8b 6b 08          	mov    0x8(%rbx),%r13
    a8ee:	49 81 fd e8 03 00 00 	cmp    $0x3e8,%r13
    a8f5:	0f 8d ec 00 00 00    	jge    a9e7 <pythrust_main+0x147>
    a8fb:	49 8d 45 01          	lea    0x1(%r13),%rax
    a8ff:	48 89 43 08          	mov    %rax,0x8(%rbx)
    a903:	4c 89 e6             	mov    %r12,%rsi
    a906:	48 89 df             	mov    %rbx,%rdi
    a909:	e8 f2 00 00 00       	call   aa00 <py1_steps>
    a90e:	4c 89 6b 08          	mov    %r13,0x8(%rbx)
    a912:	4c 89 ff             	mov    %r15,%rdi
    a915:	48 03 f8             	add    %rax,%rdi
    a918:	0f 90 c1             	seto   %cl
    a91b:	84 c9                	test   %cl,%cl
    a91d:	0f 85 ac 00 00 00    	jne    a9cf <pythrust_main+0x12f>
    a923:	4c 89 f1             	mov    %r14,%rcx
    a926:	48 3b c1             	cmp    %rcx,%rax
    a929:	0f 8e 03 00 00 00    	jle    a932 <pythrust_main+0x92>
    a92f:	49 89 c6             	mov    %rax,%r14
    a932:	4c 89 e0             	mov    %r12,%rax
    a935:	48 83 c0 01          	add    $0x1,%rax
    a939:	0f 90 c0             	seto   %al
    a93c:	84 c0                	test   %al,%al
    a93e:	0f 85 1a 00 00 00    	jne    a95e <pythrust_main+0xbe>
    a944:	49 83 c4 01          	add    $0x1,%r12
    a948:	0f 90 c0             	seto   %al
    a94b:	84 c0                	test   %al,%al
    a94d:	0f 85 65 00 00 00    	jne    a9b8 <pythrust_main+0x118>
    a953:	48 89 d8             	mov    %rbx,%rax
    a956:	49 89 ff             	mov    %rdi,%r15
    a959:	e9 74 ff ff ff       	jmp    a8d2 <pythrust_main+0x32>
    a95e:	48 8d 05 cb 08 00 00 	lea    0x8cb(%rip),%rax        # b230 <pt_write_int>
    a965:	ff d0                	call   *%rax
    a967:	b9 20 00 00 00       	mov    $0x20,%ecx
    a96c:	48 0f b6 f9          	movzbq %cl,%rdi
    a970:	48 8d 15 89 08 00 00 	lea    0x889(%rip),%rdx        # b200 <pt_write_byte>
    a977:	ff d2                	call   *%rdx
    a979:	48 8d 15 b0 08 00 00 	lea    0x8b0(%rip),%rdx        # b230 <pt_write_int>
    a980:	4c 89 f7             	mov    %r14,%rdi
    a983:	ff d2                	call   *%rdx
    a985:	be 0a 00 00 00       	mov    $0xa,%esi
    a98a:	48 0f b6 fe          	movzbq %sil,%rdi
    a98e:	48 8d 35 6b 08 00 00 	lea    0x86b(%rip),%rsi        # b200 <pt_write_byte>
    a995:	ff d6                	call   *%rsi
    a997:	48 8b 1c 24          	mov    (%rsp),%rbx
    a99b:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
    a9a0:	4c 8b 6c 24 10       	mov    0x10(%rsp),%r13
    a9a5:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
    a9aa:	4c 8b 7c 24 20       	mov    0x20(%rsp),%r15
    a9af:	48 83 c4 30          	add    $0x30,%rsp
    a9b3:	48 89 ec             	mov    %rbp,%rsp
    a9b6:	5d                   	pop    %rbp
    a9b7:	c3                   	ret
    a9b8:	48 8d 3d 76 57 03 00 	lea    0x35776(%rip),%rdi        # 40135 <.Ldata2>
    a9bf:	be aa 00 00 00       	mov    $0xaa,%esi
    a9c4:	48 8d 05 b5 07 00 00 	lea    0x7b5(%rip),%rax        # b180 <pt_fail>
    a9cb:	ff d0                	call   *%rax
    a9cd:	0f 0b                	ud2
    a9cf:	48 8d 3d cf 56 03 00 	lea    0x356cf(%rip),%rdi        # 400a5 <.Ldata1>
    a9d6:	be 90 00 00 00       	mov    $0x90,%esi
    a9db:	4c 8d 05 9e 07 00 00 	lea    0x79e(%rip),%r8        # b180 <pt_fail>
    a9e2:	41 ff d0             	call   *%r8
    a9e5:	0f 0b                	ud2
    a9e7:	48 8d 3d 12 56 03 00 	lea    0x35612(%rip),%rdi        # 40000 <.Ldata0>
    a9ee:	be a5 00 00 00       	mov    $0xa5,%esi
    a9f3:	4c 8d 05 86 07 00 00 	lea    0x786(%rip),%r8        # b180 <pt_fail>
    a9fa:	41 ff d0             	call   *%r8
    a9fd:	0f 0b                	ud2

Disassembly of section .fini:
