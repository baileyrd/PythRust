
/workspace/scratch/2ce9dd46a989/card814-candidate-final/numeric:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000a9f8 <py1_steps>:
    a9f8:	55                   	push   %rbp
    a9f9:	48 89 e5             	mov    %rsp,%rbp
    a9fc:	48 33 c0             	xor    %rax,%rax
    a9ff:	48 89 c7             	mov    %rax,%rdi
    aa02:	48 83 fe 01          	cmp    $0x1,%rsi
    aa06:	0f 85 08 00 00 00    	jne    aa14 <py1_steps+0x1c>
    aa0c:	48 89 f8             	mov    %rdi,%rax
    aa0f:	48 89 ec             	mov    %rbp,%rsp
    aa12:	5d                   	pop    %rbp
    aa13:	c3                   	ret
    aa14:	48 f7 c6 01 00 00 00 	test   $0x1,%rsi
    aa1b:	0f 84 28 00 00 00    	je     aa49 <py1_steps+0x51>
    aa21:	b8 03 00 00 00       	mov    $0x3,%eax
    aa26:	48 f7 ee             	imul   %rsi
    aa29:	0f 80 4e 00 00 00    	jo     aa7d <py1_steps+0x85>
    aa2f:	48 83 c0 01          	add    $0x1,%rax
    aa33:	0f 90 c1             	seto   %cl
    aa36:	84 c9                	test   %cl,%cl
    aa38:	0f 85 28 00 00 00    	jne    aa66 <py1_steps+0x6e>
    aa3e:	48 89 c6             	mov    %rax,%rsi
    aa41:	48 89 f8             	mov    %rdi,%rax
    aa44:	e9 06 00 00 00       	jmp    aa4f <py1_steps+0x57>
    aa49:	48 d1 fe             	sar    $1,%rsi
    aa4c:	48 89 f8             	mov    %rdi,%rax
    aa4f:	48 83 c0 01          	add    $0x1,%rax
    aa53:	0f 90 c1             	seto   %cl
    aa56:	84 c9                	test   %cl,%cl
    aa58:	0f 85 36 00 00 00    	jne    aa94 <py1_steps+0x9c>
    aa5e:	48 89 c7             	mov    %rax,%rdi
    aa61:	e9 9c ff ff ff       	jmp    aa02 <py1_steps+0xa>
    aa66:	48 8d 3d 8b 59 03 00 	lea    0x3598b(%rip),%rdi        # 403f8 <.Ldata6>
    aa6d:	be b1 00 00 00       	mov    $0xb1,%esi
    aa72:	48 8d 05 a7 06 00 00 	lea    0x6a7(%rip),%rax        # b120 <pt_fail>
    aa79:	ff d0                	call   *%rax
    aa7b:	0f 0b                	ud2
    aa7d:	48 8d 3d c7 58 03 00 	lea    0x358c7(%rip),%rdi        # 4034b <.Ldata5>
    aa84:	be ad 00 00 00       	mov    $0xad,%esi
    aa89:	48 8d 05 90 06 00 00 	lea    0x690(%rip),%rax        # b120 <pt_fail>
    aa90:	ff d0                	call   *%rax
    aa92:	0f 0b                	ud2
    aa94:	48 8d 3d 0e 58 03 00 	lea    0x3580e(%rip),%rdi        # 402a9 <.Ldata4>
    aa9b:	be a2 00 00 00       	mov    $0xa2,%esi
    aaa0:	48 8d 05 79 06 00 00 	lea    0x679(%rip),%rax        # b120 <pt_fail>
    aaa7:	ff d0                	call   *%rax
    aaa9:	0f 0b                	ud2

Disassembly of section .fini:

/workspace/scratch/2ce9dd46a989/card814-candidate-final/numeric:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000a899 <pythrust_main>:
    a899:	55                   	push   %rbp
    a89a:	48 89 e5             	mov    %rsp,%rbp
    a89d:	48 83 ec 30          	sub    $0x30,%rsp
    a8a1:	48 89 1c 24          	mov    %rbx,(%rsp)
    a8a5:	4c 89 64 24 08       	mov    %r12,0x8(%rsp)
    a8aa:	4c 89 6c 24 10       	mov    %r13,0x10(%rsp)
    a8af:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    a8b4:	4c 89 7c 24 20       	mov    %r15,0x20(%rsp)
    a8b9:	48 89 f8             	mov    %rdi,%rax
    a8bc:	41 bc 01 00 00 00    	mov    $0x1,%r12d
    a8c2:	48 33 ff             	xor    %rdi,%rdi
    a8c5:	49 89 ff             	mov    %rdi,%r15
    a8c8:	4d 89 fe             	mov    %r15,%r14
    a8cb:	49 81 fc 40 42 0f 00 	cmp    $0xf4240,%r12
    a8d2:	0f 8c 08 00 00 00    	jl     a8e0 <pythrust_main+0x47>
    a8d8:	4c 89 ff             	mov    %r15,%rdi
    a8db:	e9 77 00 00 00       	jmp    a957 <pythrust_main+0xbe>
    a8e0:	48 89 c3             	mov    %rax,%rbx
    a8e3:	4c 8b 6b 08          	mov    0x8(%rbx),%r13
    a8e7:	49 81 fd e8 03 00 00 	cmp    $0x3e8,%r13
    a8ee:	0f 8d ec 00 00 00    	jge    a9e0 <pythrust_main+0x147>
    a8f4:	49 8d 45 01          	lea    0x1(%r13),%rax
    a8f8:	48 89 43 08          	mov    %rax,0x8(%rbx)
    a8fc:	4c 89 e6             	mov    %r12,%rsi
    a8ff:	48 89 df             	mov    %rbx,%rdi
    a902:	e8 f1 00 00 00       	call   a9f8 <py1_steps>
    a907:	4c 89 6b 08          	mov    %r13,0x8(%rbx)
    a90b:	4c 89 ff             	mov    %r15,%rdi
    a90e:	48 03 f8             	add    %rax,%rdi
    a911:	0f 90 c1             	seto   %cl
    a914:	84 c9                	test   %cl,%cl
    a916:	0f 85 ac 00 00 00    	jne    a9c8 <pythrust_main+0x12f>
    a91c:	4c 89 f1             	mov    %r14,%rcx
    a91f:	48 3b c1             	cmp    %rcx,%rax
    a922:	0f 8e 03 00 00 00    	jle    a92b <pythrust_main+0x92>
    a928:	49 89 c6             	mov    %rax,%r14
    a92b:	4c 89 e0             	mov    %r12,%rax
    a92e:	48 83 c0 01          	add    $0x1,%rax
    a932:	0f 90 c0             	seto   %al
    a935:	84 c0                	test   %al,%al
    a937:	0f 85 1a 00 00 00    	jne    a957 <pythrust_main+0xbe>
    a93d:	49 83 c4 01          	add    $0x1,%r12
    a941:	0f 90 c0             	seto   %al
    a944:	84 c0                	test   %al,%al
    a946:	0f 85 65 00 00 00    	jne    a9b1 <pythrust_main+0x118>
    a94c:	48 89 d8             	mov    %rbx,%rax
    a94f:	49 89 ff             	mov    %rdi,%r15
    a952:	e9 74 ff ff ff       	jmp    a8cb <pythrust_main+0x32>
    a957:	48 8d 05 72 08 00 00 	lea    0x872(%rip),%rax        # b1d0 <pt_write_int>
    a95e:	ff d0                	call   *%rax
    a960:	b9 20 00 00 00       	mov    $0x20,%ecx
    a965:	48 0f b6 f9          	movzbq %cl,%rdi
    a969:	48 8d 15 30 08 00 00 	lea    0x830(%rip),%rdx        # b1a0 <pt_write_byte>
    a970:	ff d2                	call   *%rdx
    a972:	48 8d 15 57 08 00 00 	lea    0x857(%rip),%rdx        # b1d0 <pt_write_int>
    a979:	4c 89 f7             	mov    %r14,%rdi
    a97c:	ff d2                	call   *%rdx
    a97e:	be 0a 00 00 00       	mov    $0xa,%esi
    a983:	48 0f b6 fe          	movzbq %sil,%rdi
    a987:	48 8d 35 12 08 00 00 	lea    0x812(%rip),%rsi        # b1a0 <pt_write_byte>
    a98e:	ff d6                	call   *%rsi
    a990:	48 8b 1c 24          	mov    (%rsp),%rbx
    a994:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
    a999:	4c 8b 6c 24 10       	mov    0x10(%rsp),%r13
    a99e:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
    a9a3:	4c 8b 7c 24 20       	mov    0x20(%rsp),%r15
    a9a8:	48 83 c4 30          	add    $0x30,%rsp
    a9ac:	48 89 ec             	mov    %rbp,%rsp
    a9af:	5d                   	pop    %rbp
    a9b0:	c3                   	ret
    a9b1:	48 8d 3d 93 57 03 00 	lea    0x35793(%rip),%rdi        # 4014b <.Ldata2>
    a9b8:	be b5 00 00 00       	mov    $0xb5,%esi
    a9bd:	48 8d 05 5c 07 00 00 	lea    0x75c(%rip),%rax        # b120 <pt_fail>
    a9c4:	ff d0                	call   *%rax
    a9c6:	0f 0b                	ud2
    a9c8:	48 8d 3d e1 56 03 00 	lea    0x356e1(%rip),%rdi        # 400b0 <.Ldata1>
    a9cf:	be 9b 00 00 00       	mov    $0x9b,%esi
    a9d4:	4c 8d 05 45 07 00 00 	lea    0x745(%rip),%r8        # b120 <pt_fail>
    a9db:	41 ff d0             	call   *%r8
    a9de:	0f 0b                	ud2
    a9e0:	48 8d 3d 19 56 03 00 	lea    0x35619(%rip),%rdi        # 40000 <.Ldata0>
    a9e7:	be b0 00 00 00       	mov    $0xb0,%esi
    a9ec:	4c 8d 05 2d 07 00 00 	lea    0x72d(%rip),%r8        # b120 <pt_fail>
    a9f3:	41 ff d0             	call   *%r8
    a9f6:	0f 0b                	ud2

Disassembly of section .fini:
