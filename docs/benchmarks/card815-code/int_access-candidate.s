
/workspace/card815-revised/candidate/int_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000b8e0 <pythrust_main>:
    b8e0:	55                   	push   %rbp
    b8e1:	48 89 e5             	mov    %rsp,%rbp
    b8e4:	48 83 ec 30          	sub    $0x30,%rsp
    b8e8:	48 89 1c 24          	mov    %rbx,(%rsp)
    b8ec:	4c 89 64 24 08       	mov    %r12,0x8(%rsp)
    b8f1:	4c 89 6c 24 10       	mov    %r13,0x10(%rsp)
    b8f6:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    b8fb:	4c 89 7c 24 20       	mov    %r15,0x20(%rsp)
    b900:	48 89 fb             	mov    %rdi,%rbx
    b903:	48 8d 3d f6 36 04 00 	lea    0x436f6(%rip),%rdi        # 4f000 <.Ldata0>
    b90a:	be 02 00 00 00       	mov    $0x2,%esi
    b90f:	ba 01 00 00 00       	mov    $0x1,%edx
    b914:	48 8d 05 f5 3e 00 00 	lea    0x3ef5(%rip),%rax        # f810 <pt_seq_new>
    b91b:	ff d0                	call   *%rax
    b91d:	49 89 c5             	mov    %rax,%r13
    b920:	be 07 00 00 00       	mov    $0x7,%esi
    b925:	48 8d 05 b4 3f 00 00 	lea    0x3fb4(%rip),%rax        # f8e0 <pt_seq_push>
    b92c:	4c 89 ef             	mov    %r13,%rdi
    b92f:	ff d0                	call   *%rax
    b931:	48 8d 05 28 36 00 00 	lea    0x3628(%rip),%rax        # ef60 <pt_list_copy>
    b938:	4c 89 ef             	mov    %r13,%rdi
    b93b:	ff d0                	call   *%rax
    b93d:	49 89 c7             	mov    %rax,%r15
    b940:	be 10 00 00 00       	mov    $0x10,%esi
    b945:	48 8d 0d 74 38 00 00 	lea    0x3874(%rip),%rcx        # f1c0 <pt_list_scale>
    b94c:	4c 89 ff             	mov    %r15,%rdi
    b94f:	ff d1                	call   *%rcx
    b951:	84 c0                	test   %al,%al
    b953:	0f 85 bd 00 00 00    	jne    ba16 <pythrust_main+0x136>
    b959:	4c 8b 63 08          	mov    0x8(%rbx),%r12
    b95d:	49 81 fc e8 03 00 00 	cmp    $0x3e8,%r12
    b964:	0f 8d 95 00 00 00    	jge    b9ff <pythrust_main+0x11f>
    b96a:	49 8d 44 24 01       	lea    0x1(%r12),%rax
    b96f:	48 89 43 08          	mov    %rax,0x8(%rbx)
    b973:	4c 89 fe             	mov    %r15,%rsi
    b976:	48 89 df             	mov    %rbx,%rdi
    b979:	e8 b2 00 00 00       	call   ba30 <py1_access>
    b97e:	49 89 c6             	mov    %rax,%r14
    b981:	4c 89 63 08          	mov    %r12,0x8(%rbx)
    b985:	48 8d 05 a4 3d 00 00 	lea    0x3da4(%rip),%rax        # f730 <pt_seq_dec>
    b98c:	4c 89 ff             	mov    %r15,%rdi
    b98f:	ff d0                	call   *%rax
    b991:	48 0f b6 03          	movzbq (%rbx),%rax
    b995:	84 c0                	test   %al,%al
    b997:	0f 85 4b 00 00 00    	jne    b9e8 <pythrust_main+0x108>
    b99d:	48 8d 05 8c 68 00 00 	lea    0x688c(%rip),%rax        # 12230 <pt_write_int>
    b9a4:	4c 89 f7             	mov    %r14,%rdi
    b9a7:	ff d0                	call   *%rax
    b9a9:	b8 0a 00 00 00       	mov    $0xa,%eax
    b9ae:	48 0f b6 f8          	movzbq %al,%rdi
    b9b2:	48 8d 05 47 68 00 00 	lea    0x6847(%rip),%rax        # 12200 <pt_write_byte>
    b9b9:	ff d0                	call   *%rax
    b9bb:	48 8d 05 6e 3d 00 00 	lea    0x3d6e(%rip),%rax        # f730 <pt_seq_dec>
    b9c2:	4c 89 ef             	mov    %r13,%rdi
    b9c5:	ff d0                	call   *%rax
    b9c7:	48 8b 1c 24          	mov    (%rsp),%rbx
    b9cb:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
    b9d0:	4c 8b 6c 24 10       	mov    0x10(%rsp),%r13
    b9d5:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
    b9da:	4c 8b 7c 24 20       	mov    0x20(%rsp),%r15
    b9df:	48 83 c4 30          	add    $0x30,%rsp
    b9e3:	48 89 ec             	mov    %rbp,%rsp
    b9e6:	5d                   	pop    %rbp
    b9e7:	c3                   	ret
    b9e8:	4c 89 ef             	mov    %r13,%rdi
    b9eb:	48 8d 05 3e 3d 00 00 	lea    0x3d3e(%rip),%rax        # f730 <pt_seq_dec>
    b9f2:	ff d0                	call   *%rax
    b9f4:	48 8d 05 85 71 00 00 	lea    0x7185(%rip),%rax        # 12b80 <pt_uncaught>
    b9fb:	ff d0                	call   *%rax
    b9fd:	0f 0b                	ud2
    b9ff:	48 8d 3d a1 36 04 00 	lea    0x436a1(%rip),%rdi        # 4f0a7 <.Ldata2>
    ba06:	be aa 00 00 00       	mov    $0xaa,%esi
    ba0b:	48 8d 05 8e 67 00 00 	lea    0x678e(%rip),%rax        # 121a0 <pt_fail>
    ba12:	ff d0                	call   *%rax
    ba14:	0f 0b                	ud2
    ba16:	48 8d 3d e5 35 04 00 	lea    0x435e5(%rip),%rdi        # 4f002 <.Ldata1>
    ba1d:	be a5 00 00 00       	mov    $0xa5,%esi
    ba22:	48 8d 05 77 67 00 00 	lea    0x6777(%rip),%rax        # 121a0 <pt_fail>
    ba29:	ff d0                	call   *%rax
    ba2b:	0f 0b                	ud2

Disassembly of section .fini:

/workspace/card815-revised/candidate/int_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000ee80 <pt_int_list_get>:
    ee80:	53                   	push   %rbx
    ee81:	48 83 ec 20          	sub    $0x20,%rsp
    ee85:	48 89 d3             	mov    %rdx,%rbx
    ee88:	48 89 f2             	mov    %rsi,%rdx
    ee8b:	48 89 fe             	mov    %rdi,%rsi
    ee8e:	48 8b 47 18          	mov    0x18(%rdi),%rax
    ee92:	48 85 c0             	test   %rax,%rax
    ee95:	75 33                	jne    eeca <pt_int_list_get+0x4a>
    ee97:	48 8b 4e 30          	mov    0x30(%rsi),%rcx
    ee9b:	48 89 d0             	mov    %rdx,%rax
    ee9e:	48 c1 f8 3f          	sar    $0x3f,%rax
    eea2:	48 21 c8             	and    %rcx,%rax
    eea5:	48 01 d0             	add    %rdx,%rax
    eea8:	0f 99 c2             	setns  %dl
    eeab:	48 39 c8             	cmp    %rcx,%rax
    eeae:	0f 92 c1             	setb   %cl
    eeb1:	20 d1                	and    %dl,%cl
    eeb3:	80 f9 01             	cmp    $0x1,%cl
    eeb6:	75 2e                	jne    eee6 <pt_int_list_get+0x66>
    eeb8:	48 8b 4e 28          	mov    0x28(%rsi),%rcx
    eebc:	48 8b 04 c1          	mov    (%rcx,%rax,8),%rax
    eec0:	31 c9                	xor    %ecx,%ecx
    eec2:	88 0b                	mov    %cl,(%rbx)
    eec4:	48 83 c4 20          	add    $0x20,%rsp
    eec8:	5b                   	pop    %rbx
    eec9:	c3                   	ret
    eeca:	48 83 c6 18          	add    $0x18,%rsi
    eece:	48 8d 7c 24 08       	lea    0x8(%rsp),%rdi
    eed3:	e8 88 71 ff ff       	call   6060 <_RINvMs1_NtCsk1kE6r7WBHz_11pythrust_rt4syncINtB6_4SlotINtNtCs40k4W9msRzi_5alloc3vec3VecyEE12get_borrowedNCNvNtB8_3seq15pt_int_list_get0EB8_>
    eed8:	83 7c 24 08 01       	cmpl   $0x1,0x8(%rsp)
    eedd:	75 07                	jne    eee6 <pt_int_list_get+0x66>
    eedf:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
    eee4:	eb da                	jmp    eec0 <pt_int_list_get+0x40>
    eee6:	b1 01                	mov    $0x1,%cl
    eee8:	31 c0                	xor    %eax,%eax
    eeea:	88 0b                	mov    %cl,(%rbx)
    eeec:	48 83 c4 20          	add    $0x20,%rsp
    eef0:	5b                   	pop    %rbx
    eef1:	c3                   	ret
    eef2:	ff 15 90 49 05 00    	call   *0x54990(%rip)        # 63888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card815-revised/candidate/int_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000ef00 <pt_int_list_set>:
    ef00:	50                   	push   %rax
    ef01:	48 8b 47 18          	mov    0x18(%rdi),%rax
    ef05:	48 85 c0             	test   %rax,%rax
    ef08:	75 33                	jne    ef3d <pt_int_list_set+0x3d>
    ef0a:	48 8b 47 30          	mov    0x30(%rdi),%rax
    ef0e:	48 89 f1             	mov    %rsi,%rcx
    ef11:	48 c1 f9 3f          	sar    $0x3f,%rcx
    ef15:	48 21 c1             	and    %rax,%rcx
    ef18:	48 01 f1             	add    %rsi,%rcx
    ef1b:	40 0f 99 c6          	setns  %sil
    ef1f:	48 39 c1             	cmp    %rax,%rcx
    ef22:	41 0f 92 c0          	setb   %r8b
    ef26:	41 20 f0             	and    %sil,%r8b
    ef29:	b0 01                	mov    $0x1,%al
    ef2b:	41 80 f8 01          	cmp    $0x1,%r8b
    ef2f:	75 0a                	jne    ef3b <pt_int_list_set+0x3b>
    ef31:	48 8b 47 28          	mov    0x28(%rdi),%rax
    ef35:	48 89 14 c8          	mov    %rdx,(%rax,%rcx,8)
    ef39:	31 c0                	xor    %eax,%eax
    ef3b:	59                   	pop    %rcx
    ef3c:	c3                   	ret
    ef3d:	48 83 c7 18          	add    $0x18,%rdi
    ef41:	e8 da 72 ff ff       	call   6220 <_RINvMs1_NtCsk1kE6r7WBHz_11pythrust_rt4syncINtB6_4SlotINtNtCs40k4W9msRzi_5alloc3vec3VecyEE16replace_borrowedNCNvNtB8_3seq15pt_int_list_set0EB8_>
    ef46:	48 83 f8 01          	cmp    $0x1,%rax
    ef4a:	0f 95 c0             	setne  %al
    ef4d:	59                   	pop    %rcx
    ef4e:	c3                   	ret
    ef4f:	ff 15 33 49 05 00    	call   *0x54933(%rip)        # 63888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card815-revised/candidate/int_access:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000ba30 <py1_access>:
    ba30:	55                   	push   %rbp
    ba31:	48 89 e5             	mov    %rsp,%rbp
    ba34:	48 83 ec 50          	sub    $0x50,%rsp
    ba38:	48 89 5c 24 20       	mov    %rbx,0x20(%rsp)
    ba3d:	4c 89 64 24 28       	mov    %r12,0x28(%rsp)
    ba42:	4c 89 6c 24 30       	mov    %r13,0x30(%rsp)
    ba47:	4c 89 74 24 38       	mov    %r14,0x38(%rsp)
    ba4c:	4c 89 7c 24 40       	mov    %r15,0x40(%rsp)
    ba51:	49 89 f6             	mov    %rsi,%r14
    ba54:	48 33 db             	xor    %rbx,%rbx
    ba57:	4c 8d 64 24 10       	lea    0x10(%rsp),%r12
    ba5c:	49 89 df             	mov    %rbx,%r15
    ba5f:	48 81 fb 00 09 3d 00 	cmp    $0x3d0900,%rbx
    ba66:	0f 8d b7 00 00 00    	jge    bb23 <py1_access+0xf3>
    ba6c:	48 89 d8             	mov    %rbx,%rax
    ba6f:	48 c1 f8 03          	sar    $0x3,%rax
    ba73:	48 c1 e8 3c          	shr    $0x3c,%rax
    ba77:	48 03 c3             	add    %rbx,%rax
    ba7a:	48 83 e0 f0          	and    $0xfffffffffffffff0,%rax
    ba7e:	48 89 d9             	mov    %rbx,%rcx
    ba81:	48 2b c8             	sub    %rax,%rcx
    ba84:	48 3b d8             	cmp    %rax,%rbx
    ba87:	0f 95 c2             	setne  %dl
    ba8a:	48 89 ce             	mov    %rcx,%rsi
    ba8d:	48 83 f6 10          	xor    $0x10,%rsi
    ba91:	48 33 c0             	xor    %rax,%rax
    ba94:	48 c1 ee 3f          	shr    $0x3f,%rsi
    ba98:	40 84 d6             	test   %dl,%sil
    ba9b:	48 0f 45 05 5d 01 00 	cmovne 0x15d(%rip),%rax        # bc00 <py1_access+0x1d0>
    baa2:	00 
    baa3:	4c 8d 2c 01          	lea    (%rcx,%rax,1),%r13
    baa7:	4c 8d 05 d2 33 00 00 	lea    0x33d2(%rip),%r8        # ee80 <pt_int_list_get>
    baae:	4c 89 e2             	mov    %r12,%rdx
    bab1:	4c 89 ee             	mov    %r13,%rsi
    bab4:	4c 89 f7             	mov    %r14,%rdi
    bab7:	41 ff d0             	call   *%r8
    baba:	48 0f b6 54 24 10    	movzbq 0x10(%rsp),%rdx
    bac0:	40 80 fa 01          	rex cmp $0x1,%dl
    bac4:	0f 84 f5 00 00 00    	je     bbbf <py1_access+0x18f>
    baca:	4c 89 f9             	mov    %r15,%rcx
    bacd:	48 03 c8             	add    %rax,%rcx
    bad0:	49 89 cf             	mov    %rcx,%r15
    bad3:	40 0f 90 c6          	seto   %sil
    bad7:	40 84 f6             	test   %sil,%sil
    bada:	0f 85 c8 00 00 00    	jne    bba8 <py1_access+0x178>
    bae0:	ba 07 00 00 00       	mov    $0x7,%edx
    bae5:	4c 8d 05 14 34 00 00 	lea    0x3414(%rip),%r8        # ef00 <pt_int_list_set>
    baec:	4c 89 ee             	mov    %r13,%rsi
    baef:	4c 89 f7             	mov    %r14,%rdi
    baf2:	41 ff d0             	call   *%r8
    baf5:	3c 01                	cmp    $0x1,%al
    baf7:	0f 84 6a 00 00 00    	je     bb67 <py1_access+0x137>
    bafd:	49 89 db             	mov    %rbx,%r11
    bb00:	49 83 c3 01          	add    $0x1,%r11
    bb04:	0f 90 c0             	seto   %al
    bb07:	84 c0                	test   %al,%al
    bb09:	0f 85 14 00 00 00    	jne    bb23 <py1_access+0xf3>
    bb0f:	48 83 c3 01          	add    $0x1,%rbx
    bb13:	0f 90 c0             	seto   %al
    bb16:	84 c0                	test   %al,%al
    bb18:	0f 85 32 00 00 00    	jne    bb50 <py1_access+0x120>
    bb1e:	e9 3c ff ff ff       	jmp    ba5f <py1_access+0x2f>
    bb23:	4c 89 f8             	mov    %r15,%rax
    bb26:	e9 03 00 00 00       	jmp    bb2e <py1_access+0xfe>
    bb2b:	48 33 c0             	xor    %rax,%rax
    bb2e:	48 8b 5c 24 20       	mov    0x20(%rsp),%rbx
    bb33:	4c 8b 64 24 28       	mov    0x28(%rsp),%r12
    bb38:	4c 8b 6c 24 30       	mov    0x30(%rsp),%r13
    bb3d:	4c 8b 74 24 38       	mov    0x38(%rsp),%r14
    bb42:	4c 8b 7c 24 40       	mov    0x40(%rsp),%r15
    bb47:	48 83 c4 50          	add    $0x50,%rsp
    bb4b:	48 89 ec             	mov    %rbp,%rsp
    bb4e:	5d                   	pop    %rbp
    bb4f:	c3                   	ret
    bb50:	48 8d 3d 15 37 04 00 	lea    0x43715(%rip),%rdi        # 4f26c <.Ldata9>
    bb57:	be a3 00 00 00       	mov    $0xa3,%esi
    bb5c:	48 8d 05 3d 66 00 00 	lea    0x663d(%rip),%rax        # 121a0 <pt_fail>
    bb63:	ff d0                	call   *%rax
    bb65:	0f 0b                	ud2
    bb67:	b8 01 00 00 00       	mov    $0x1,%eax
    bb6c:	48 8d 35 f0 38 04 00 	lea    0x438f0(%rip),%rsi        # 4f463 <.Ldatac>
    bb73:	ba 22 00 00 00       	mov    $0x22,%edx
    bb78:	48 8d 0d 06 39 04 00 	lea    0x43906(%rip),%rcx        # 4f485 <.Ldatad>
    bb7f:	41 b8 45 00 00 00    	mov    $0x45,%r8d
    bb85:	4c 8d 0d 3e 39 04 00 	lea    0x4393e(%rip),%r9        # 4f4ca <.Ldatae>
    bb8c:	41 ba 25 00 00 00    	mov    $0x25,%r10d
    bb92:	48 0f b6 f8          	movzbq %al,%rdi
    bb96:	4c 89 14 24          	mov    %r10,(%rsp)
    bb9a:	48 8d 05 ef 6e 00 00 	lea    0x6eef(%rip),%rax        # 12a90 <pt_raise_msg>
    bba1:	ff d0                	call   *%rax
    bba3:	e9 83 ff ff ff       	jmp    bb2b <py1_access+0xfb>
    bba8:	48 8d 3d 60 37 04 00 	lea    0x43760(%rip),%rdi        # 4f30f <.Ldataa>
    bbaf:	be 8f 00 00 00       	mov    $0x8f,%esi
    bbb4:	48 8d 05 e5 65 00 00 	lea    0x65e5(%rip),%rax        # 121a0 <pt_fail>
    bbbb:	ff d0                	call   *%rax
    bbbd:	0f 0b                	ud2
    bbbf:	b8 01 00 00 00       	mov    $0x1,%eax
    bbc4:	48 8d 35 19 36 04 00 	lea    0x43619(%rip),%rsi        # 4f1e4 <.Ldata6>
    bbcb:	ba 17 00 00 00       	mov    $0x17,%edx
    bbd0:	48 8d 0d 24 36 04 00 	lea    0x43624(%rip),%rcx        # 4f1fb <.Ldata7>
    bbd7:	41 b8 46 00 00 00    	mov    $0x46,%r8d
    bbdd:	4c 8d 0d 5d 36 04 00 	lea    0x4365d(%rip),%r9        # 4f241 <.Ldata8>
    bbe4:	41 ba 2b 00 00 00    	mov    $0x2b,%r10d
    bbea:	48 0f b6 f8          	movzbq %al,%rdi
    bbee:	4c 89 14 24          	mov    %r10,(%rsp)
    bbf2:	48 8d 05 97 6e 00 00 	lea    0x6e97(%rip),%rax        # 12a90 <pt_raise_msg>
    bbf9:	ff d0                	call   *%rax
    bbfb:	e9 2b ff ff ff       	jmp    bb2b <py1_access+0xfb>
    bc00:	10 00                	adc    %al,(%rax)
    bc02:	00 00                	add    %al,(%rax)
    bc04:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:
