
/workspace/scratch/2ce9dd46a989/card814-controls/tuples_source_inline:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000ace8 <pt_scalar_pair_1>:
    ace8:	55                   	push   %rbp
    ace9:	48 89 e5             	mov    %rsp,%rbp
    acec:	48 33 c0             	xor    %rax,%rax
    acef:	49 89 c2             	mov    %rax,%r10
    acf2:	4c 89 d2             	mov    %r10,%rdx
    acf5:	48 85 f6             	test   %rsi,%rsi
    acf8:	0f 8f 08 00 00 00    	jg     ad06 <pt_scalar_pair_1+0x1e>
    acfe:	4c 89 d0             	mov    %r10,%rax
    ad01:	48 89 ec             	mov    %rbp,%rsp
    ad04:	5d                   	pop    %rbp
    ad05:	c3                   	ret
    ad06:	4c 89 d0             	mov    %r10,%rax
    ad09:	49 89 d3             	mov    %rdx,%r11
    ad0c:	48 83 c0 01          	add    $0x1,%rax
    ad10:	49 89 c2             	mov    %rax,%r10
    ad13:	0f 90 c0             	seto   %al
    ad16:	84 c0                	test   %al,%al
    ad18:	0f 85 82 00 00 00    	jne    ada0 <pt_scalar_pair_1+0xb8>
    ad1e:	48 89 f0             	mov    %rsi,%rax
    ad21:	48 f7 2d 90 00 00 00 	imulq  0x90(%rip)        # adb8 <pt_scalar_pair_1+0xd0>
    ad28:	48 c1 fa 02          	sar    $0x2,%rdx
    ad2c:	48 89 d0             	mov    %rdx,%rax
    ad2f:	48 c1 e8 3f          	shr    $0x3f,%rax
    ad33:	48 03 c2             	add    %rdx,%rax
    ad36:	48 6b d0 0a          	imul   $0xa,%rax,%rdx
    ad3a:	48 89 f1             	mov    %rsi,%rcx
    ad3d:	48 2b ca             	sub    %rdx,%rcx
    ad40:	48 3b f2             	cmp    %rdx,%rsi
    ad43:	40 0f 95 c7          	setne  %dil
    ad47:	49 89 c9             	mov    %rcx,%r9
    ad4a:	49 83 f1 0a          	xor    $0xa,%r9
    ad4e:	4d 33 c0             	xor    %r8,%r8
    ad51:	49 c1 e9 3f          	shr    $0x3f,%r9
    ad55:	48 89 fe             	mov    %rdi,%rsi
    ad58:	41 23 f1             	and    %r9d,%esi
    ad5b:	41 84 f9             	test   %dil,%r9b
    ad5e:	4c 0f 45 05 5a 00 00 	cmovne 0x5a(%rip),%r8        # adc0 <pt_scalar_pair_1+0xd8>
    ad65:	00 
    ad66:	49 03 c8             	add    %r8,%rcx
    ad69:	4c 89 da             	mov    %r11,%rdx
    ad6c:	48 03 d1             	add    %rcx,%rdx
    ad6f:	0f 90 c1             	seto   %cl
    ad72:	84 c9                	test   %cl,%cl
    ad74:	0f 85 0f 00 00 00    	jne    ad89 <pt_scalar_pair_1+0xa1>
    ad7a:	48 0f b6 ce          	movzbq %sil,%rcx
    ad7e:	48 2b c1             	sub    %rcx,%rax
    ad81:	48 89 c6             	mov    %rax,%rsi
    ad84:	e9 6c ff ff ff       	jmp    acf5 <pt_scalar_pair_1+0xd>
    ad89:	48 8d 3d e1 67 03 00 	lea    0x367e1(%rip),%rdi        # 41571 <.Ldatab>
    ad90:	be a9 00 00 00       	mov    $0xa9,%esi
    ad95:	48 8d 05 64 14 00 00 	lea    0x1464(%rip),%rax        # c200 <pt_fail>
    ad9c:	ff d0                	call   *%rax
    ad9e:	0f 0b                	ud2
    ada0:	48 8d 3d 21 67 03 00 	lea    0x36721(%rip),%rdi        # 414c8 <.Ldataa>
    ada7:	be a9 00 00 00       	mov    $0xa9,%esi
    adac:	48 8d 05 4d 14 00 00 	lea    0x144d(%rip),%rax        # c200 <pt_fail>
    adb3:	ff d0                	call   *%rax
    adb5:	0f 0b                	ud2
    adb7:	00 67 66             	add    %ah,0x66(%rdi)
    adba:	66 66 66 66 66 66 0a 	data16 data16 data16 data16 data16 data16 or (%rax),%al
    adc1:	00 
    adc2:	00 00                	add    %al,(%rax)
    adc4:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:

/workspace/scratch/2ce9dd46a989/card814-controls/tuples_source_inline:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000aba8 <py1_digits>:
    aba8:	55                   	push   %rbp
    aba9:	48 89 e5             	mov    %rsp,%rbp
    abac:	48 83 ec 20          	sub    $0x20,%rsp
    abb0:	48 89 1c 24          	mov    %rbx,(%rsp)
    abb4:	4c 89 64 24 08       	mov    %r12,0x8(%rsp)
    abb9:	4c 89 6c 24 10       	mov    %r13,0x10(%rsp)
    abbe:	48 33 c0             	xor    %rax,%rax
    abc1:	48 89 c3             	mov    %rax,%rbx
    abc4:	49 89 dc             	mov    %rbx,%r12
    abc7:	48 85 f6             	test   %rsi,%rsi
    abca:	0f 8f 55 00 00 00    	jg     ac25 <py1_digits+0x7d>
    abd0:	48 8d 3d 43 6a 03 00 	lea    0x36a43(%rip),%rdi        # 4161a <.Ldatac>
    abd7:	be 04 00 00 00       	mov    $0x4,%esi
    abdc:	ba 02 00 00 00       	mov    $0x2,%edx
    abe1:	48 8d 05 b8 0d 00 00 	lea    0xdb8(%rip),%rax        # b9a0 <pt_seq_new>
    abe8:	ff d0                	call   *%rax
    abea:	49 89 c5             	mov    %rax,%r13
    abed:	48 8d 05 7c 0e 00 00 	lea    0xe7c(%rip),%rax        # ba70 <pt_seq_push>
    abf4:	48 89 de             	mov    %rbx,%rsi
    abf7:	4c 89 ef             	mov    %r13,%rdi
    abfa:	ff d0                	call   *%rax
    abfc:	48 8d 05 6d 0e 00 00 	lea    0xe6d(%rip),%rax        # ba70 <pt_seq_push>
    ac03:	4c 89 e6             	mov    %r12,%rsi
    ac06:	4c 89 ef             	mov    %r13,%rdi
    ac09:	ff d0                	call   *%rax
    ac0b:	4c 89 e8             	mov    %r13,%rax
    ac0e:	48 8b 1c 24          	mov    (%rsp),%rbx
    ac12:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
    ac17:	4c 8b 6c 24 10       	mov    0x10(%rsp),%r13
    ac1c:	48 83 c4 20          	add    $0x20,%rsp
    ac20:	48 89 ec             	mov    %rbp,%rsp
    ac23:	5d                   	pop    %rbp
    ac24:	c3                   	ret
    ac25:	48 89 df             	mov    %rbx,%rdi
    ac28:	48 83 c7 01          	add    $0x1,%rdi
    ac2c:	0f 90 c0             	seto   %al
    ac2f:	84 c0                	test   %al,%al
    ac31:	0f 85 88 00 00 00    	jne    acbf <py1_digits+0x117>
    ac37:	48 89 f0             	mov    %rsi,%rax
    ac3a:	48 f7 2d 97 00 00 00 	imulq  0x97(%rip)        # acd8 <py1_digits+0x130>
    ac41:	48 c1 fa 02          	sar    $0x2,%rdx
    ac45:	48 89 d0             	mov    %rdx,%rax
    ac48:	48 c1 e8 3f          	shr    $0x3f,%rax
    ac4c:	48 03 c2             	add    %rdx,%rax
    ac4f:	48 6b d0 0a          	imul   $0xa,%rax,%rdx
    ac53:	48 89 f1             	mov    %rsi,%rcx
    ac56:	48 2b ca             	sub    %rdx,%rcx
    ac59:	48 3b f2             	cmp    %rdx,%rsi
    ac5c:	40 0f 95 c6          	setne  %sil
    ac60:	49 89 c9             	mov    %rcx,%r9
    ac63:	49 83 f1 0a          	xor    $0xa,%r9
    ac67:	4d 33 c0             	xor    %r8,%r8
    ac6a:	49 c1 e9 3f          	shr    $0x3f,%r9
    ac6e:	48 89 f2             	mov    %rsi,%rdx
    ac71:	41 23 d1             	and    %r9d,%edx
    ac74:	41 84 f1             	test   %sil,%r9b
    ac77:	4c 0f 45 05 61 00 00 	cmovne 0x61(%rip),%r8        # ace0 <py1_digits+0x138>
    ac7e:	00 
    ac7f:	49 03 c8             	add    %r8,%rcx
    ac82:	4c 89 e6             	mov    %r12,%rsi
    ac85:	48 03 f1             	add    %rcx,%rsi
    ac88:	0f 90 c1             	seto   %cl
    ac8b:	84 c9                	test   %cl,%cl
    ac8d:	0f 85 15 00 00 00    	jne    aca8 <py1_digits+0x100>
    ac93:	48 0f b6 ca          	movzbq %dl,%rcx
    ac97:	48 2b c1             	sub    %rcx,%rax
    ac9a:	49 89 f4             	mov    %rsi,%r12
    ac9d:	48 89 c6             	mov    %rax,%rsi
    aca0:	48 89 fb             	mov    %rdi,%rbx
    aca3:	e9 1f ff ff ff       	jmp    abc7 <py1_digits+0x1f>
    aca8:	48 8d 3d c2 68 03 00 	lea    0x368c2(%rip),%rdi        # 41571 <.Ldatab>
    acaf:	be a9 00 00 00       	mov    $0xa9,%esi
    acb4:	48 8d 05 45 15 00 00 	lea    0x1545(%rip),%rax        # c200 <pt_fail>
    acbb:	ff d0                	call   *%rax
    acbd:	0f 0b                	ud2
    acbf:	48 8d 3d 02 68 03 00 	lea    0x36802(%rip),%rdi        # 414c8 <.Ldataa>
    acc6:	be a9 00 00 00       	mov    $0xa9,%esi
    accb:	48 8d 05 2e 15 00 00 	lea    0x152e(%rip),%rax        # c200 <pt_fail>
    acd2:	ff d0                	call   *%rax
    acd4:	0f 0b                	ud2
    acd6:	00 00                	add    %al,(%rax)
    acd8:	67 66 66 66 66 66 66 	data16 data16 data16 data16 data16 data16 data16 or (%eax),%al
    acdf:	66 0a 00 
    ace2:	00 00                	add    %al,(%rax)
    ace4:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:

/workspace/scratch/2ce9dd46a989/card814-controls/tuples_source_inline:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000aa30 <pythrust_main>:
    aa30:	55                   	push   %rbp
    aa31:	48 89 e5             	mov    %rsp,%rbp
    aa34:	48 83 ec 30          	sub    $0x30,%rsp
    aa38:	48 89 1c 24          	mov    %rbx,(%rsp)
    aa3c:	4c 89 64 24 08       	mov    %r12,0x8(%rsp)
    aa41:	4c 89 6c 24 10       	mov    %r13,0x10(%rsp)
    aa46:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    aa4b:	4c 89 7c 24 20       	mov    %r15,0x20(%rsp)
    aa50:	4d 33 ed             	xor    %r13,%r13
    aa53:	4d 89 ee             	mov    %r13,%r14
    aa56:	4d 89 ef             	mov    %r13,%r15
    aa59:	49 81 fd 40 42 0f 00 	cmp    $0xf4240,%r13
    aa60:	0f 8c 08 00 00 00    	jl     aa6e <pythrust_main+0x3e>
    aa66:	4c 89 ff             	mov    %r15,%rdi
    aa69:	e9 7d 00 00 00       	jmp    aaeb <pythrust_main+0xbb>
    aa6e:	48 89 fb             	mov    %rdi,%rbx
    aa71:	4c 8b 63 08          	mov    0x8(%rbx),%r12
    aa75:	49 81 fc e8 03 00 00 	cmp    $0x3e8,%r12
    aa7c:	0f 8d 0a 01 00 00    	jge    ab8c <pythrust_main+0x15c>
    aa82:	49 8d 44 24 01       	lea    0x1(%r12),%rax
    aa87:	48 89 43 08          	mov    %rax,0x8(%rbx)
    aa8b:	4c 89 ee             	mov    %r13,%rsi
    aa8e:	48 89 df             	mov    %rbx,%rdi
    aa91:	e8 52 02 00 00       	call   ace8 <pt_scalar_pair_1>
    aa96:	4c 89 63 08          	mov    %r12,0x8(%rbx)
    aa9a:	4c 89 ff             	mov    %r15,%rdi
    aa9d:	48 03 f8             	add    %rax,%rdi
    aaa0:	0f 90 c0             	seto   %al
    aaa3:	84 c0                	test   %al,%al
    aaa5:	0f 85 c9 00 00 00    	jne    ab74 <pythrust_main+0x144>
    aaab:	4c 89 f0             	mov    %r14,%rax
    aaae:	48 03 c2             	add    %rdx,%rax
    aab1:	49 89 c6             	mov    %rax,%r14
    aab4:	0f 90 c0             	seto   %al
    aab7:	84 c0                	test   %al,%al
    aab9:	0f 85 9d 00 00 00    	jne    ab5c <pythrust_main+0x12c>
    aabf:	4c 89 e8             	mov    %r13,%rax
    aac2:	48 83 c0 01          	add    $0x1,%rax
    aac6:	0f 90 c0             	seto   %al
    aac9:	84 c0                	test   %al,%al
    aacb:	0f 85 1a 00 00 00    	jne    aaeb <pythrust_main+0xbb>
    aad1:	49 83 c5 01          	add    $0x1,%r13
    aad5:	0f 90 c0             	seto   %al
    aad8:	84 c0                	test   %al,%al
    aada:	0f 85 65 00 00 00    	jne    ab45 <pythrust_main+0x115>
    aae0:	49 89 ff             	mov    %rdi,%r15
    aae3:	48 89 df             	mov    %rbx,%rdi
    aae6:	e9 6e ff ff ff       	jmp    aa59 <pythrust_main+0x29>
    aaeb:	48 8d 05 be 17 00 00 	lea    0x17be(%rip),%rax        # c2b0 <pt_write_int>
    aaf2:	ff d0                	call   *%rax
    aaf4:	b8 20 00 00 00       	mov    $0x20,%eax
    aaf9:	48 0f b6 f8          	movzbq %al,%rdi
    aafd:	48 8d 05 7c 17 00 00 	lea    0x177c(%rip),%rax        # c280 <pt_write_byte>
    ab04:	ff d0                	call   *%rax
    ab06:	48 8d 05 a3 17 00 00 	lea    0x17a3(%rip),%rax        # c2b0 <pt_write_int>
    ab0d:	4c 89 f7             	mov    %r14,%rdi
    ab10:	ff d0                	call   *%rax
    ab12:	b9 0a 00 00 00       	mov    $0xa,%ecx
    ab17:	48 0f b6 f9          	movzbq %cl,%rdi
    ab1b:	48 8d 15 5e 17 00 00 	lea    0x175e(%rip),%rdx        # c280 <pt_write_byte>
    ab22:	ff d2                	call   *%rdx
    ab24:	48 8b 1c 24          	mov    (%rsp),%rbx
    ab28:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
    ab2d:	4c 8b 6c 24 10       	mov    0x10(%rsp),%r13
    ab32:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
    ab37:	4c 8b 7c 24 20       	mov    0x20(%rsp),%r15
    ab3c:	48 83 c4 30          	add    $0x30,%rsp
    ab40:	48 89 ec             	mov    %rbp,%rsp
    ab43:	5d                   	pop    %rbp
    ab44:	c3                   	ret
    ab45:	48 8d 3d c5 66 03 00 	lea    0x366c5(%rip),%rdi        # 41211 <.Ldata3>
    ab4c:	be b5 00 00 00       	mov    $0xb5,%esi
    ab51:	48 8d 05 a8 16 00 00 	lea    0x16a8(%rip),%rax        # c200 <pt_fail>
    ab58:	ff d0                	call   *%rax
    ab5a:	0f 0b                	ud2
    ab5c:	48 8d 3d 05 66 03 00 	lea    0x36605(%rip),%rdi        # 41168 <.Ldata2>
    ab63:	be a9 00 00 00       	mov    $0xa9,%esi
    ab68:	4c 8d 05 91 16 00 00 	lea    0x1691(%rip),%r8        # c200 <pt_fail>
    ab6f:	41 ff d0             	call   *%r8
    ab72:	0f 0b                	ud2
    ab74:	48 8d 3d 44 65 03 00 	lea    0x36544(%rip),%rdi        # 410bf <.Ldata1>
    ab7b:	be a9 00 00 00       	mov    $0xa9,%esi
    ab80:	4c 8d 05 79 16 00 00 	lea    0x1679(%rip),%r8        # c200 <pt_fail>
    ab87:	41 ff d0             	call   *%r8
    ab8a:	0f 0b                	ud2
    ab8c:	48 8d 3d 6d 64 03 00 	lea    0x3646d(%rip),%rdi        # 41000 <.Ldata0>
    ab93:	be bf 00 00 00       	mov    $0xbf,%esi
    ab98:	4c 8d 05 61 16 00 00 	lea    0x1661(%rip),%r8        # c200 <pt_fail>
    ab9f:	41 ff d0             	call   *%r8
    aba2:	0f 0b                	ud2

Disassembly of section .fini:
