
/workspace/card815-run/baseline/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000b9c0 <pythrust_main>:
    b9c0:	55                   	push   %rbp
    b9c1:	48 89 e5             	mov    %rsp,%rbp
    b9c4:	48 81 ec 80 00 00 00 	sub    $0x80,%rsp
    b9cb:	48 89 5c 24 50       	mov    %rbx,0x50(%rsp)
    b9d0:	4c 89 64 24 58       	mov    %r12,0x58(%rsp)
    b9d5:	4c 89 6c 24 60       	mov    %r13,0x60(%rsp)
    b9da:	4c 89 74 24 68       	mov    %r14,0x68(%rsp)
    b9df:	4c 89 7c 24 70       	mov    %r15,0x70(%rsp)
    b9e4:	49 89 fd             	mov    %rdi,%r13
    b9e7:	48 8d 3d 12 36 04 00 	lea    0x43612(%rip),%rdi        # 4f000 <.Ldata0>
    b9ee:	be 02 00 00 00       	mov    $0x2,%esi
    b9f3:	48 33 db             	xor    %rbx,%rbx
    b9f6:	4c 8d 05 73 45 00 00 	lea    0x4573(%rip),%r8        # ff70 <pt_seq_new>
    b9fd:	48 89 da             	mov    %rbx,%rdx
    ba00:	41 ff d0             	call   *%r8
    ba03:	48 89 44 24 48       	mov    %rax,0x48(%rsp)
    ba08:	b8 39 30 00 00       	mov    $0x3039,%eax
    ba0d:	48 81 fb e0 93 04 00 	cmp    $0x493e0,%rbx
    ba14:	0f 8d e2 00 00 00    	jge    bafc <pythrust_main+0x13c>
    ba1a:	48 f7 2d 9f 05 00 00 	imulq  0x59f(%rip)        # bfc0 <pythrust_main+0x600>
    ba21:	0f 80 7e 05 00 00    	jo     bfa5 <pythrust_main+0x5e5>
    ba27:	48 05 39 30 00 00    	add    $0x3039,%rax
    ba2d:	41 0f 90 c0          	seto   %r8b
    ba31:	45 84 c0             	test   %r8b,%r8b
    ba34:	0f 85 54 05 00 00    	jne    bf8e <pythrust_main+0x5ce>
    ba3a:	49 89 c2             	mov    %rax,%r10
    ba3d:	49 c1 fa 1e          	sar    $0x1e,%r10
    ba41:	49 c1 ea 21          	shr    $0x21,%r10
    ba45:	4c 03 d0             	add    %rax,%r10
    ba48:	49 81 e2 00 00 00 80 	and    $0xffffffff80000000,%r10
    ba4f:	49 89 c3             	mov    %rax,%r11
    ba52:	4d 2b da             	sub    %r10,%r11
    ba55:	49 3b c2             	cmp    %r10,%rax
    ba58:	0f 95 c0             	setne  %al
    ba5b:	4d 89 d9             	mov    %r11,%r9
    ba5e:	4c 33 0d 63 05 00 00 	xor    0x563(%rip),%r9        # bfc8 <pythrust_main+0x608>
    ba65:	48 33 c9             	xor    %rcx,%rcx
    ba68:	49 c1 e9 3f          	shr    $0x3f,%r9
    ba6c:	41 84 c1             	test   %al,%r9b
    ba6f:	48 89 c8             	mov    %rcx,%rax
    ba72:	48 0f 45 05 4e 05 00 	cmovne 0x54e(%rip),%rax        # bfc8 <pythrust_main+0x608>
    ba79:	00 
    ba7a:	49 03 c3             	add    %r11,%rax
    ba7d:	49 89 c4             	mov    %rax,%r12
    ba80:	48 f7 2d 49 05 00 00 	imulq  0x549(%rip)        # bfd0 <pythrust_main+0x610>
    ba87:	48 c1 fa 12          	sar    $0x12,%rdx
    ba8b:	48 89 d0             	mov    %rdx,%rax
    ba8e:	48 c1 e8 3f          	shr    $0x3f,%rax
    ba92:	48 03 c2             	add    %rdx,%rax
    ba95:	48 69 d0 40 42 0f 00 	imul   $0xf4240,%rax,%rdx
    ba9c:	4c 89 e0             	mov    %r12,%rax
    ba9f:	48 2b c2             	sub    %rdx,%rax
    baa2:	4c 3b e2             	cmp    %rdx,%r12
    baa5:	0f 95 c2             	setne  %dl
    baa8:	48 89 c6             	mov    %rax,%rsi
    baab:	48 81 f6 40 42 0f 00 	xor    $0xf4240,%rsi
    bab2:	48 c1 ee 3f          	shr    $0x3f,%rsi
    bab6:	40 84 d6             	test   %dl,%sil
    bab9:	48 0f 45 0d 17 05 00 	cmovne 0x517(%rip),%rcx        # bfd8 <pythrust_main+0x618>
    bac0:	00 
    bac1:	48 8d 34 08          	lea    (%rax,%rcx,1),%rsi
    bac5:	48 8d 05 74 45 00 00 	lea    0x4574(%rip),%rax        # 10040 <pt_seq_push>
    bacc:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bad1:	ff d0                	call   *%rax
    bad3:	48 89 d8             	mov    %rbx,%rax
    bad6:	48 83 c0 01          	add    $0x1,%rax
    bada:	0f 90 c0             	seto   %al
    badd:	84 c0                	test   %al,%al
    badf:	0f 85 17 00 00 00    	jne    bafc <pythrust_main+0x13c>
    bae5:	48 83 c3 01          	add    $0x1,%rbx
    bae9:	0f 90 c0             	seto   %al
    baec:	84 c0                	test   %al,%al
    baee:	0f 85 9b 02 00 00    	jne    bd8f <pythrust_main+0x3cf>
    baf4:	4c 89 e0             	mov    %r12,%rax
    baf7:	e9 11 ff ff ff       	jmp    ba0d <pythrust_main+0x4d>
    bafc:	48 8d 05 ed 43 00 00 	lea    0x43ed(%rip),%rax        # fef0 <pt_seq_len>
    bb03:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bb08:	ff d0                	call   *%rax
    bb0a:	48 89 c1             	mov    %rax,%rcx
    bb0d:	48 83 e9 01          	sub    $0x1,%rcx
    bb11:	0f 90 c0             	seto   %al
    bb14:	84 c0                	test   %al,%al
    bb16:	0f 85 5b 04 00 00    	jne    bf77 <pythrust_main+0x5b7>
    bb1c:	4d 89 ef             	mov    %r13,%r15
    bb1f:	49 8b 5f 08          	mov    0x8(%r15),%rbx
    bb23:	48 81 fb e8 03 00 00 	cmp    $0x3e8,%rbx
    bb2a:	0f 8d 30 04 00 00    	jge    bf60 <pythrust_main+0x5a0>
    bb30:	48 8d 43 01          	lea    0x1(%rbx),%rax
    bb34:	49 89 47 08          	mov    %rax,0x8(%r15)
    bb38:	48 33 d2             	xor    %rdx,%rdx
    bb3b:	48 8b 74 24 48       	mov    0x48(%rsp),%rsi
    bb40:	4c 89 ff             	mov    %r15,%rdi
    bb43:	e8 98 04 00 00       	call   bfe0 <py1_quicksort>
    bb48:	49 89 5f 08          	mov    %rbx,0x8(%r15)
    bb4c:	49 0f b6 07          	movzbq (%r15),%rax
    bb50:	84 c0                	test   %al,%al
    bb52:	0f 85 ef 03 00 00    	jne    bf47 <pythrust_main+0x587>
    bb58:	48 8d 05 91 43 00 00 	lea    0x4391(%rip),%rax        # fef0 <pt_seq_len>
    bb5f:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bb64:	ff d0                	call   *%rax
    bb66:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
    bb6b:	41 bc 01 00 00 00    	mov    $0x1,%r12d
    bb71:	bb 01 00 00 00       	mov    $0x1,%ebx
    bb76:	4c 8d 6c 24 10       	lea    0x10(%rsp),%r13
    bb7b:	4c 8d 74 24 18       	lea    0x18(%rsp),%r14
    bb80:	48 8b 4c 24 40       	mov    0x40(%rsp),%rcx
    bb85:	49 89 df             	mov    %rbx,%r15
    bb88:	4c 3b e1             	cmp    %rcx,%r12
    bb8b:	48 89 4c 24 40       	mov    %rcx,0x40(%rsp)
    bb90:	0f 8d 9b 00 00 00    	jge    bc31 <pythrust_main+0x271>
    bb96:	4c 89 e6             	mov    %r12,%rsi
    bb99:	48 83 ee 01          	sub    $0x1,%rsi
    bb9d:	0f 90 c0             	seto   %al
    bba0:	84 c0                	test   %al,%al
    bba2:	0f 85 88 03 00 00    	jne    bf30 <pythrust_main+0x570>
    bba8:	48 8d 05 a1 42 00 00 	lea    0x42a1(%rip),%rax        # fe50 <pt_seq_get>
    bbaf:	4c 89 ea             	mov    %r13,%rdx
    bbb2:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bbb7:	ff d0                	call   *%rax
    bbb9:	48 89 44 24 38       	mov    %rax,0x38(%rsp)
    bbbe:	48 0f b6 44 24 10    	movzbq 0x10(%rsp),%rax
    bbc4:	3c 01                	cmp    $0x1,%al
    bbc6:	0f 84 23 03 00 00    	je     beef <pythrust_main+0x52f>
    bbcc:	48 8d 05 7d 42 00 00 	lea    0x427d(%rip),%rax        # fe50 <pt_seq_get>
    bbd3:	4c 89 f2             	mov    %r14,%rdx
    bbd6:	4c 89 e6             	mov    %r12,%rsi
    bbd9:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bbde:	ff d0                	call   *%rax
    bbe0:	48 0f b6 4c 24 18    	movzbq 0x18(%rsp),%rcx
    bbe6:	40 80 f9 01          	rex cmp $0x1,%cl
    bbea:	0f 84 be 02 00 00    	je     beae <pythrust_main+0x4ee>
    bbf0:	4c 8b 5c 24 38       	mov    0x38(%rsp),%r11
    bbf5:	4c 3b d8             	cmp    %rax,%r11
    bbf8:	0f 8e 08 00 00 00    	jle    bc06 <pythrust_main+0x246>
    bbfe:	bb 01 00 00 00       	mov    $0x1,%ebx
    bc03:	45 33 ff             	xor    %r15d,%r15d
    bc06:	4c 89 e0             	mov    %r12,%rax
    bc09:	48 83 c0 01          	add    $0x1,%rax
    bc0d:	0f 90 c0             	seto   %al
    bc10:	84 c0                	test   %al,%al
    bc12:	0f 85 19 00 00 00    	jne    bc31 <pythrust_main+0x271>
    bc18:	49 83 c4 01          	add    $0x1,%r12
    bc1c:	0f 90 c0             	seto   %al
    bc1f:	84 c0                	test   %al,%al
    bc21:	0f 85 7f 01 00 00    	jne    bda6 <pythrust_main+0x3e6>
    bc27:	48 8b 4c 24 40       	mov    0x40(%rsp),%rcx
    bc2c:	e9 57 ff ff ff       	jmp    bb88 <pythrust_main+0x1c8>
    bc31:	84 db                	test   %bl,%bl
    bc33:	0f 84 5e 02 00 00    	je     be97 <pythrust_main+0x4d7>
    bc39:	48 33 f6             	xor    %rsi,%rsi
    bc3c:	48 8d 54 24 20       	lea    0x20(%rsp),%rdx
    bc41:	48 8d 0d 08 42 00 00 	lea    0x4208(%rip),%rcx        # fe50 <pt_seq_get>
    bc48:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bc4d:	ff d1                	call   *%rcx
    bc4f:	49 89 c6             	mov    %rax,%r14
    bc52:	48 0f b6 4c 24 20    	movzbq 0x20(%rsp),%rcx
    bc58:	40 80 f9 01          	rex cmp $0x1,%cl
    bc5c:	0f 84 f4 01 00 00    	je     be56 <pythrust_main+0x496>
    bc62:	48 8d 35 87 42 00 00 	lea    0x4287(%rip),%rsi        # fef0 <pt_seq_len>
    bc69:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bc6e:	ff d6                	call   *%rsi
    bc70:	48 89 c6             	mov    %rax,%rsi
    bc73:	48 d1 fe             	sar    $1,%rsi
    bc76:	48 8d 54 24 28       	lea    0x28(%rsp),%rdx
    bc7b:	4c 8d 05 ce 41 00 00 	lea    0x41ce(%rip),%r8        # fe50 <pt_seq_get>
    bc82:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bc87:	41 ff d0             	call   *%r8
    bc8a:	48 89 c3             	mov    %rax,%rbx
    bc8d:	48 0f b6 74 24 28    	movzbq 0x28(%rsp),%rsi
    bc93:	40 80 fe 01          	cmp    $0x1,%sil
    bc97:	0f 84 78 01 00 00    	je     be15 <pythrust_main+0x455>
    bc9d:	4c 8d 05 4c 42 00 00 	lea    0x424c(%rip),%r8        # fef0 <pt_seq_len>
    bca4:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bca9:	41 ff d0             	call   *%r8
    bcac:	48 89 c6             	mov    %rax,%rsi
    bcaf:	48 83 ee 01          	sub    $0x1,%rsi
    bcb3:	41 0f 90 c0          	seto   %r8b
    bcb7:	45 84 c0             	test   %r8b,%r8b
    bcba:	0f 85 3e 01 00 00    	jne    bdfe <pythrust_main+0x43e>
    bcc0:	48 8d 54 24 30       	lea    0x30(%rsp),%rdx
    bcc5:	4c 8d 0d 84 41 00 00 	lea    0x4184(%rip),%r9        # fe50 <pt_seq_get>
    bccc:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bcd1:	41 ff d1             	call   *%r9
    bcd4:	49 89 c4             	mov    %rax,%r12
    bcd7:	4c 0f b6 4c 24 30    	movzbq 0x30(%rsp),%r9
    bcdd:	41 80 f9 01          	cmp    $0x1,%r9b
    bce1:	0f 84 d6 00 00 00    	je     bdbd <pythrust_main+0x3fd>
    bce7:	49 0f b6 ff          	movzbq %r15b,%rdi
    bceb:	48 8d 05 6e 6c 00 00 	lea    0x6c6e(%rip),%rax        # 12960 <pt_write_bool>
    bcf2:	ff d0                	call   *%rax
    bcf4:	41 bd 20 00 00 00    	mov    $0x20,%r13d
    bcfa:	49 0f b6 fd          	movzbq %r13b,%rdi
    bcfe:	48 8d 05 9b 6c 00 00 	lea    0x6c9b(%rip),%rax        # 129a0 <pt_write_byte>
    bd05:	ff d0                	call   *%rax
    bd07:	48 8d 05 c2 6c 00 00 	lea    0x6cc2(%rip),%rax        # 129d0 <pt_write_int>
    bd0e:	4c 89 f7             	mov    %r14,%rdi
    bd11:	ff d0                	call   *%rax
    bd13:	49 0f b6 fd          	movzbq %r13b,%rdi
    bd17:	48 8d 05 82 6c 00 00 	lea    0x6c82(%rip),%rax        # 129a0 <pt_write_byte>
    bd1e:	ff d0                	call   *%rax
    bd20:	48 8d 05 a9 6c 00 00 	lea    0x6ca9(%rip),%rax        # 129d0 <pt_write_int>
    bd27:	48 89 df             	mov    %rbx,%rdi
    bd2a:	ff d0                	call   *%rax
    bd2c:	49 0f b6 fd          	movzbq %r13b,%rdi
    bd30:	48 8d 05 69 6c 00 00 	lea    0x6c69(%rip),%rax        # 129a0 <pt_write_byte>
    bd37:	ff d0                	call   *%rax
    bd39:	48 8d 05 90 6c 00 00 	lea    0x6c90(%rip),%rax        # 129d0 <pt_write_int>
    bd40:	4c 89 e7             	mov    %r12,%rdi
    bd43:	ff d0                	call   *%rax
    bd45:	b8 0a 00 00 00       	mov    $0xa,%eax
    bd4a:	48 0f b6 f8          	movzbq %al,%rdi
    bd4e:	48 8d 05 4b 6c 00 00 	lea    0x6c4b(%rip),%rax        # 129a0 <pt_write_byte>
    bd55:	ff d0                	call   *%rax
    bd57:	48 8d 05 12 40 00 00 	lea    0x4012(%rip),%rax        # fd70 <pt_seq_dec>
    bd5e:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bd63:	ff d0                	call   *%rax
    bd65:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
    bd6a:	4c 8b 64 24 58       	mov    0x58(%rsp),%r12
    bd6f:	4c 8b 6c 24 60       	mov    0x60(%rsp),%r13
    bd74:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
    bd79:	4c 8b 7c 24 70       	mov    0x70(%rsp),%r15
    bd7e:	48 81 c4 80 00 00 00 	add    $0x80,%rsp
    bd85:	48 89 ec             	mov    %rbp,%rsp
    bd88:	5d                   	pop    %rbp
    bd89:	c3                   	ret
    bd8a:	e9 b8 01 00 00       	jmp    bf47 <pythrust_main+0x587>
    bd8f:	48 8d 3d 30 35 04 00 	lea    0x43530(%rip),%rdi        # 4f2c6 <.Ldata8>
    bd96:	be 93 00 00 00       	mov    $0x93,%esi
    bd9b:	48 8d 05 5e 6b 00 00 	lea    0x6b5e(%rip),%rax        # 12900 <pt_fail>
    bda2:	ff d0                	call   *%rax
    bda4:	0f 0b                	ud2
    bda6:	48 8d 3d 0f 38 04 00 	lea    0x4380f(%rip),%rdi        # 4f5bc <.Ldataf>
    bdad:	be 9b 00 00 00       	mov    $0x9b,%esi
    bdb2:	48 8d 05 47 6b 00 00 	lea    0x6b47(%rip),%rax        # 12900 <pt_fail>
    bdb9:	ff d0                	call   *%rax
    bdbb:	0f 0b                	ud2
    bdbd:	b8 01 00 00 00       	mov    $0x1,%eax
    bdc2:	48 8d 35 6d 37 04 00 	lea    0x4376d(%rip),%rsi        # 4f536 <.Ldatac>
    bdc9:	ba 17 00 00 00       	mov    $0x17,%edx
    bdce:	48 8d 0d 25 3c 04 00 	lea    0x43c25(%rip),%rcx        # 4f9fa <.Ldata19>
    bdd5:	41 b8 40 00 00 00    	mov    $0x40,%r8d
    bddb:	4c 8d 0d 58 3c 04 00 	lea    0x43c58(%rip),%r9        # 4fa3a <.Ldata1a>
    bde2:	41 ba 6c 00 00 00    	mov    $0x6c,%r10d
    bde8:	48 0f b6 f8          	movzbq %al,%rdi
    bdec:	4c 89 14 24          	mov    %r10,(%rsp)
    bdf0:	48 8d 05 39 74 00 00 	lea    0x7439(%rip),%rax        # 13230 <pt_raise_msg>
    bdf7:	ff d0                	call   *%rax
    bdf9:	e9 49 01 00 00       	jmp    bf47 <pythrust_main+0x587>
    bdfe:	48 8d 3d 2b 3b 04 00 	lea    0x43b2b(%rip),%rdi        # 4f930 <.Ldata18>
    be05:	be ca 00 00 00       	mov    $0xca,%esi
    be0a:	48 8d 05 ef 6a 00 00 	lea    0x6aef(%rip),%rax        # 12900 <pt_fail>
    be11:	ff d0                	call   *%rax
    be13:	0f 0b                	ud2
    be15:	b8 01 00 00 00       	mov    $0x1,%eax
    be1a:	48 8d 35 15 37 04 00 	lea    0x43715(%rip),%rsi        # 4f536 <.Ldatac>
    be21:	ba 17 00 00 00       	mov    $0x17,%edx
    be26:	48 8d 0d 68 3a 04 00 	lea    0x43a68(%rip),%rcx        # 4f895 <.Ldata16>
    be2d:	41 b8 40 00 00 00    	mov    $0x40,%r8d
    be33:	4c 8d 0d 9b 3a 04 00 	lea    0x43a9b(%rip),%r9        # 4f8d5 <.Ldata17>
    be3a:	41 ba 5b 00 00 00    	mov    $0x5b,%r10d
    be40:	48 0f b6 f8          	movzbq %al,%rdi
    be44:	4c 89 14 24          	mov    %r10,(%rsp)
    be48:	48 8d 05 e1 73 00 00 	lea    0x73e1(%rip),%rax        # 13230 <pt_raise_msg>
    be4f:	ff d0                	call   *%rax
    be51:	e9 f1 00 00 00       	jmp    bf47 <pythrust_main+0x587>
    be56:	b8 01 00 00 00       	mov    $0x1,%eax
    be5b:	48 8d 35 d4 36 04 00 	lea    0x436d4(%rip),%rsi        # 4f536 <.Ldatac>
    be62:	ba 17 00 00 00       	mov    $0x17,%edx
    be67:	48 8d 0d 9e 39 04 00 	lea    0x4399e(%rip),%rcx        # 4f80c <.Ldata14>
    be6e:	41 b8 40 00 00 00    	mov    $0x40,%r8d
    be74:	4c 8d 0d d1 39 04 00 	lea    0x439d1(%rip),%r9        # 4f84c <.Ldata15>
    be7b:	41 ba 49 00 00 00    	mov    $0x49,%r10d
    be81:	48 0f b6 f8          	movzbq %al,%rdi
    be85:	4c 89 14 24          	mov    %r10,(%rsp)
    be89:	48 8d 05 a0 73 00 00 	lea    0x73a0(%rip),%rax        # 13230 <pt_raise_msg>
    be90:	ff d0                	call   *%rax
    be92:	e9 b0 00 00 00       	jmp    bf47 <pythrust_main+0x587>
    be97:	48 8d 3d ca 38 04 00 	lea    0x438ca(%rip),%rdi        # 4f768 <.Ldata13>
    be9e:	be a4 00 00 00       	mov    $0xa4,%esi
    bea3:	48 8d 05 56 6a 00 00 	lea    0x6a56(%rip),%rax        # 12900 <pt_fail>
    beaa:	ff d0                	call   *%rax
    beac:	0f 0b                	ud2
    beae:	b8 01 00 00 00       	mov    $0x1,%eax
    beb3:	48 8d 35 7c 36 04 00 	lea    0x4367c(%rip),%rsi        # 4f536 <.Ldatac>
    beba:	ba 17 00 00 00       	mov    $0x17,%edx
    bebf:	48 8d 0d 2a 38 04 00 	lea    0x4382a(%rip),%rcx        # 4f6f0 <.Ldata11>
    bec6:	41 b8 40 00 00 00    	mov    $0x40,%r8d
    becc:	4c 8d 0d 5d 38 04 00 	lea    0x4385d(%rip),%r9        # 4f730 <.Ldata12>
    bed3:	41 ba 38 00 00 00    	mov    $0x38,%r10d
    bed9:	48 0f b6 f8          	movzbq %al,%rdi
    bedd:	4c 89 14 24          	mov    %r10,(%rsp)
    bee1:	48 8d 05 48 73 00 00 	lea    0x7348(%rip),%rax        # 13230 <pt_raise_msg>
    bee8:	ff d0                	call   *%rax
    beea:	e9 58 00 00 00       	jmp    bf47 <pythrust_main+0x587>
    beef:	b8 01 00 00 00       	mov    $0x1,%eax
    bef4:	48 8d 35 3b 36 04 00 	lea    0x4363b(%rip),%rsi        # 4f536 <.Ldatac>
    befb:	ba 17 00 00 00       	mov    $0x17,%edx
    bf00:	48 8d 0d 46 36 04 00 	lea    0x43646(%rip),%rcx        # 4f54d <.Ldatad>
    bf07:	41 b8 3f 00 00 00    	mov    $0x3f,%r8d
    bf0d:	4c 8d 0d 78 36 04 00 	lea    0x43678(%rip),%r9        # 4f58c <.Ldatae>
    bf14:	41 ba 30 00 00 00    	mov    $0x30,%r10d
    bf1a:	48 0f b6 f8          	movzbq %al,%rdi
    bf1e:	4c 89 14 24          	mov    %r10,(%rsp)
    bf22:	48 8d 05 07 73 00 00 	lea    0x7307(%rip),%rax        # 13230 <pt_raise_msg>
    bf29:	ff d0                	call   *%rax
    bf2b:	e9 17 00 00 00       	jmp    bf47 <pythrust_main+0x587>
    bf30:	48 8d 3d 71 35 04 00 	lea    0x43571(%rip),%rdi        # 4f4a8 <.Ldatab>
    bf37:	be 8e 00 00 00       	mov    $0x8e,%esi
    bf3c:	48 8d 05 bd 69 00 00 	lea    0x69bd(%rip),%rax        # 12900 <pt_fail>
    bf43:	ff d0                	call   *%rax
    bf45:	0f 0b                	ud2
    bf47:	48 8d 05 22 3e 00 00 	lea    0x3e22(%rip),%rax        # fd70 <pt_seq_dec>
    bf4e:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bf53:	ff d0                	call   *%rax
    bf55:	48 8d 05 c4 73 00 00 	lea    0x73c4(%rip),%rax        # 13320 <pt_uncaught>
    bf5c:	ff d0                	call   *%rax
    bf5e:	0f 0b                	ud2
    bf60:	48 8d 3d 91 34 04 00 	lea    0x43491(%rip),%rdi        # 4f3f8 <.Ldataa>
    bf67:	be b0 00 00 00       	mov    $0xb0,%esi
    bf6c:	48 8d 05 8d 69 00 00 	lea    0x698d(%rip),%rax        # 12900 <pt_fail>
    bf73:	ff d0                	call   *%rax
    bf75:	0f 0b                	ud2
    bf77:	48 8d 3d db 33 04 00 	lea    0x433db(%rip),%rdi        # 4f359 <.Ldata9>
    bf7e:	be 9f 00 00 00       	mov    $0x9f,%esi
    bf83:	48 8d 05 76 69 00 00 	lea    0x6976(%rip),%rax        # 12900 <pt_fail>
    bf8a:	ff d0                	call   *%rax
    bf8c:	0f 0b                	ud2
    bf8e:	48 8d 3d 23 31 04 00 	lea    0x43123(%rip),%rdi        # 4f0b8 <.Ldata2>
    bf95:	be bf 00 00 00       	mov    $0xbf,%esi
    bf9a:	48 8d 05 5f 69 00 00 	lea    0x695f(%rip),%rax        # 12900 <pt_fail>
    bfa1:	ff d0                	call   *%rax
    bfa3:	0f 0b                	ud2
    bfa5:	48 8d 3d 56 30 04 00 	lea    0x43056(%rip),%rdi        # 4f002 <.Ldata1>
    bfac:	be b6 00 00 00       	mov    $0xb6,%esi
    bfb1:	48 8d 05 48 69 00 00 	lea    0x6948(%rip),%rax        # 12900 <pt_fail>
    bfb8:	ff d0                	call   *%rax
    bfba:	0f 0b                	ud2
    bfbc:	00 00                	add    %al,(%rax)
    bfbe:	00 00                	add    %al,(%rax)
    bfc0:	6d                   	insl   (%dx),%es:(%rdi)
    bfc1:	4e c6 41 00 00       	rex.WRX movb $0x0,0x0(%rcx)
    bfc6:	00 00                	add    %al,(%rax)
    bfc8:	00 00                	add    %al,(%rax)
    bfca:	00 80 00 00 00 00    	add    %al,0x0(%rax)
    bfd0:	db 34 b6             	(bad) (%rsi,%rsi,4)
    bfd3:	d7                   	xlat   %ds:(%rbx)
    bfd4:	82                   	(bad)
    bfd5:	de 1b                	ficomps (%rbx)
    bfd7:	43                   	rex.XB
    bfd8:	40                   	rex
    bfd9:	42 0f 00 00          	rex.X sldt (%rax)
    bfdd:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:

/workspace/card815-run/baseline/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000fe50 <pt_seq_get>:
    fe50:	41 56                	push   %r14
    fe52:	53                   	push   %rbx
    fe53:	48 83 ec 18          	sub    $0x18,%rsp
    fe57:	48 89 d3             	mov    %rdx,%rbx
    fe5a:	48 89 f9             	mov    %rdi,%rcx
    fe5d:	48 8b 47 18          	mov    0x18(%rdi),%rax
    fe61:	48 85 c0             	test   %rax,%rax
    fe64:	75 48                	jne    feae <pt_seq_get+0x5e>
    fe66:	48 8b 41 30          	mov    0x30(%rcx),%rax
    fe6a:	48 89 f2             	mov    %rsi,%rdx
    fe6d:	48 c1 fa 3f          	sar    $0x3f,%rdx
    fe71:	48 21 c2             	and    %rax,%rdx
    fe74:	48 01 f2             	add    %rsi,%rdx
    fe77:	40 0f 99 c6          	setns  %sil
    fe7b:	48 39 c2             	cmp    %rax,%rdx
    fe7e:	0f 92 c0             	setb   %al
    fe81:	40 20 f0             	and    %sil,%al
    fe84:	3c 01                	cmp    $0x1,%al
    fe86:	75 47                	jne    fecf <pt_seq_get+0x7f>
    fe88:	48 8b 41 28          	mov    0x28(%rcx),%rax
    fe8c:	4c 8b 34 d0          	mov    (%rax,%rdx,8),%r14
    fe90:	48 8b 39             	mov    (%rcx),%rdi
    fe93:	48 8b 71 08          	mov    0x8(%rcx),%rsi
    fe97:	e8 74 f5 ff ff       	call   f410 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq4slot>
    fe9c:	48 89 c7             	mov    %rax,%rdi
    fe9f:	48 89 d6             	mov    %rdx,%rsi
    fea2:	4c 89 f2             	mov    %r14,%rdx
    fea5:	e8 96 d4 ff ff       	call   d340 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq3inc>
    feaa:	31 c0                	xor    %eax,%eax
    feac:	eb 26                	jmp    fed4 <pt_seq_get+0x84>
    feae:	48 8d 41 18          	lea    0x18(%rcx),%rax
    feb2:	48 89 e7             	mov    %rsp,%rdi
    feb5:	48 89 f2             	mov    %rsi,%rdx
    feb8:	48 89 c6             	mov    %rax,%rsi
    febb:	e8 a0 61 ff ff       	call   6060 <_RINvMs1_NtCsk1kE6r7WBHz_11pythrust_rt4syncINtB6_4SlotINtNtCs40k4W9msRzi_5alloc3vec3VecyEE12get_borrowedNCNvNtB8_3seq10pt_seq_get0EB8_>
    fec0:	83 3c 24 01          	cmpl   $0x1,(%rsp)
    fec4:	75 09                	jne    fecf <pt_seq_get+0x7f>
    fec6:	4c 8b 74 24 10       	mov    0x10(%rsp),%r14
    fecb:	31 c0                	xor    %eax,%eax
    fecd:	eb 05                	jmp    fed4 <pt_seq_get+0x84>
    fecf:	b0 01                	mov    $0x1,%al
    fed1:	45 31 f6             	xor    %r14d,%r14d
    fed4:	88 03                	mov    %al,(%rbx)
    fed6:	4c 89 f0             	mov    %r14,%rax
    fed9:	48 83 c4 18          	add    $0x18,%rsp
    fedd:	5b                   	pop    %rbx
    fede:	41 5e                	pop    %r14
    fee0:	c3                   	ret
    fee1:	ff 15 a1 59 05 00    	call   *0x559a1(%rip)        # 65888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card815-run/baseline/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000fb30 <pt_list_set>:
    fb30:	41 57                	push   %r15
    fb32:	41 56                	push   %r14
    fb34:	53                   	push   %rbx
    fb35:	48 83 ec 40          	sub    $0x40,%rsp
    fb39:	48 89 54 24 20       	mov    %rdx,0x20(%rsp)
    fb3e:	48 8b 07             	mov    (%rdi),%rax
    fb41:	48 8b 4f 08          	mov    0x8(%rdi),%rcx
    fb45:	48 85 c9             	test   %rcx,%rcx
    fb48:	74 2d                	je     fb77 <pt_list_set+0x47>
    fb4a:	44 0f b6 00          	movzbl (%rax),%r8d
    fb4e:	41 83 c0 9c          	add    $0xffffff9c,%r8d
    fb52:	41 83 f8 14          	cmp    $0x14,%r8d
    fb56:	77 1f                	ja     fb77 <pt_list_set+0x47>
    fb58:	4c 8d 0d 2d 1e 04 00 	lea    0x41e2d(%rip),%r9        # 5198c <.Ldata49+0x3cb>
    fb5f:	4f 63 04 81          	movslq (%r9,%r8,4),%r8
    fb63:	4d 01 c8             	add    %r9,%r8
    fb66:	41 ff e0             	jmp    *%r8
    fb69:	48 ff c9             	dec    %rcx
    fb6c:	48 ff c0             	inc    %rax
    fb6f:	48 89 cb             	mov    %rcx,%rbx
    fb72:	49 89 c6             	mov    %rax,%r14
    fb75:	eb 18                	jmp    fb8f <pt_list_set+0x5f>
    fb77:	48 83 c0 02          	add    $0x2,%rax
    fb7b:	31 db                	xor    %ebx,%ebx
    fb7d:	48 83 e9 02          	sub    $0x2,%rcx
    fb81:	41 be 01 00 00 00    	mov    $0x1,%r14d
    fb87:	4c 0f 43 f0          	cmovae %rax,%r14
    fb8b:	48 0f 43 d9          	cmovae %rcx,%rbx
    fb8f:	4c 89 34 24          	mov    %r14,(%rsp)
    fb93:	48 89 5c 24 08       	mov    %rbx,0x8(%rsp)
    fb98:	48 8d 44 24 20       	lea    0x20(%rsp),%rax
    fb9d:	48 89 44 24 10       	mov    %rax,0x10(%rsp)
    fba2:	48 8b 47 18          	mov    0x18(%rdi),%rax
    fba6:	48 85 c0             	test   %rax,%rax
    fba9:	0f 85 57 01 00 00    	jne    fd06 <pt_list_set+0x1d6>
    fbaf:	48 8b 4f 30          	mov    0x30(%rdi),%rcx
    fbb3:	48 89 f0             	mov    %rsi,%rax
    fbb6:	48 c1 f8 3f          	sar    $0x3f,%rax
    fbba:	48 21 c8             	and    %rcx,%rax
    fbbd:	48 01 f0             	add    %rsi,%rax
    fbc0:	40 0f 99 c6          	setns  %sil
    fbc4:	48 39 c8             	cmp    %rcx,%rax
    fbc7:	0f 92 c1             	setb   %cl
    fbca:	40 84 ce             	test   %cl,%sil
    fbcd:	0f 84 b5 00 00 00    	je     fc88 <pt_list_set+0x158>
    fbd3:	48 8b 4f 28          	mov    0x28(%rdi),%rcx
    fbd7:	4c 8b 3c c1          	mov    (%rcx,%rax,8),%r15
    fbdb:	48 89 14 c1          	mov    %rdx,(%rcx,%rax,8)
    fbdf:	48 8b 54 24 20       	mov    0x20(%rsp),%rdx
    fbe4:	4c 89 f7             	mov    %r14,%rdi
    fbe7:	48 89 de             	mov    %rbx,%rsi
    fbea:	e8 51 d7 ff ff       	call   d340 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq3inc>
    fbef:	48 c7 44 24 28 00 00 	movq   $0x0,0x28(%rsp)
    fbf6:	00 00 
    fbf8:	48 c7 44 24 30 08 00 	movq   $0x8,0x30(%rsp)
    fbff:	00 00 
    fc01:	48 c7 44 24 38 00 00 	movq   $0x0,0x38(%rsp)
    fc08:	00 00 
    fc0a:	48 8d 4c 24 28       	lea    0x28(%rsp),%rcx
    fc0f:	4c 89 f7             	mov    %r14,%rdi
    fc12:	48 89 de             	mov    %rbx,%rsi
    fc15:	4c 89 fa             	mov    %r15,%rdx
    fc18:	e8 33 fc ff ff       	call   f850 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq8dec_into.llvm.12450914586151300226>
    fc1d:	48 8b 44 24 38       	mov    0x38(%rsp),%rax
    fc22:	48 89 44 24 10       	mov    %rax,0x10(%rsp)
    fc27:	0f 10 44 24 28       	movups 0x28(%rsp),%xmm0
    fc2c:	0f 29 04 24          	movaps %xmm0,(%rsp)
    fc30:	48 89 e3             	mov    %rsp,%rbx
    fc33:	66 66 66 66 2e 0f 1f 	data16 data16 data16 cs nopw 0x0(%rax,%rax,1)
    fc3a:	84 00 00 00 00 00 
    fc40:	48 8b 44 24 10       	mov    0x10(%rsp),%rax
    fc45:	48 85 c0             	test   %rax,%rax
    fc48:	74 1d                	je     fc67 <pt_list_set+0x137>
    fc4a:	48 8d 48 ff          	lea    -0x1(%rax),%rcx
    fc4e:	48 89 4c 24 10       	mov    %rcx,0x10(%rsp)
    fc53:	48 8b 4c 24 08       	mov    0x8(%rsp),%rcx
    fc58:	48 8b 7c c1 f8       	mov    -0x8(%rcx,%rax,8),%rdi
    fc5d:	48 89 de             	mov    %rbx,%rsi
    fc60:	e8 4b fd ff ff       	call   f9b0 <_RNvNtCsk1kE6r7WBHz_11pythrust_rt3seq8drop_one.llvm.12450914586151300226>
    fc65:	eb d9                	jmp    fc40 <pt_list_set+0x110>
    fc67:	48 8b 34 24          	mov    (%rsp),%rsi
    fc6b:	48 85 f6             	test   %rsi,%rsi
    fc6e:	74 14                	je     fc84 <pt_list_set+0x154>
    fc70:	48 8b 7c 24 08       	mov    0x8(%rsp),%rdi
    fc75:	48 c1 e6 03          	shl    $0x3,%rsi
    fc79:	ba 08 00 00 00       	mov    $0x8,%edx
    fc7e:	ff 15 a4 62 05 00    	call   *0x562a4(%rip)        # 65f28 <_DYNAMIC+0x9c8>
    fc84:	31 c0                	xor    %eax,%eax
    fc86:	eb 02                	jmp    fc8a <pt_list_set+0x15a>
    fc88:	b0 01                	mov    $0x1,%al
    fc8a:	48 83 c4 40          	add    $0x40,%rsp
    fc8e:	5b                   	pop    %rbx
    fc8f:	41 5e                	pop    %r14
    fc91:	41 5f                	pop    %r15
    fc93:	c3                   	ret
    fc94:	48 83 c0 02          	add    $0x2,%rax
    fc98:	45 31 c0             	xor    %r8d,%r8d
    fc9b:	48 83 f9 01          	cmp    $0x1,%rcx
    fc9f:	48 0f 44 c1          	cmove  %rcx,%rax
    fca3:	48 8d 49 fe          	lea    -0x2(%rcx),%rcx
    fca7:	49 0f 44 c8          	cmove  %r8,%rcx
    fcab:	48 85 c9             	test   %rcx,%rcx
    fcae:	74 43                	je     fcf3 <pt_list_set+0x1c3>
    fcb0:	4c 8d 48 01          	lea    0x1(%rax),%r9
    fcb4:	48 8d 59 ff          	lea    -0x1(%rcx),%rbx
    fcb8:	45 31 c0             	xor    %r8d,%r8d
    fcbb:	80 38 45             	cmpb   $0x45,(%rax)
    fcbe:	4d 0f 44 c1          	cmove  %r9,%r8
    fcc2:	eb 2f                	jmp    fcf3 <pt_list_set+0x1c3>
    fcc4:	4c 8d 35 bb 4e 04 00 	lea    0x44ebb(%rip),%r14        # 54b86 <anon.b3c5fbecbf30bc3e25184a8a8c95e8a7.9.llvm.12450914586151300226>
    fccb:	bb 01 00 00 00       	mov    $0x1,%ebx
    fcd0:	e9 ba fe ff ff       	jmp    fb8f <pt_list_set+0x5f>
    fcd5:	4c 8d 41 fe          	lea    -0x2(%rcx),%r8
    fcd9:	48 83 c0 02          	add    $0x2,%rax
    fcdd:	31 db                	xor    %ebx,%ebx
    fcdf:	48 83 f9 01          	cmp    $0x1,%rcx
    fce3:	49 0f 45 d8          	cmovne %r8,%rbx
    fce7:	48 0f 44 c1          	cmove  %rcx,%rax
    fceb:	49 89 c6             	mov    %rax,%r14
    fcee:	e9 9c fe ff ff       	jmp    fb8f <pt_list_set+0x5f>
    fcf3:	4d 85 c0             	test   %r8,%r8
    fcf6:	48 0f 44 d9          	cmove  %rcx,%rbx
    fcfa:	49 0f 45 c0          	cmovne %r8,%rax
    fcfe:	49 89 c6             	mov    %rax,%r14
    fd01:	e9 89 fe ff ff       	jmp    fb8f <pt_list_set+0x5f>
    fd06:	48 83 c7 18          	add    $0x18,%rdi
    fd0a:	48 89 e1             	mov    %rsp,%rcx
    fd0d:	e8 6e 65 ff ff       	call   6280 <_RINvMs1_NtCsk1kE6r7WBHz_11pythrust_rt4syncINtB6_4SlotINtNtCs40k4W9msRzi_5alloc3vec3VecyEE16replace_borrowedNCNvNtB8_3seq11pt_list_set0EB8_>
    fd12:	48 89 c1             	mov    %rax,%rcx
    fd15:	b0 01                	mov    $0x1,%al
    fd17:	48 83 f9 01          	cmp    $0x1,%rcx
    fd1b:	0f 85 69 ff ff ff    	jne    fc8a <pt_list_set+0x15a>
    fd21:	49 89 d7             	mov    %rdx,%r15
    fd24:	e9 c6 fe ff ff       	jmp    fbef <pt_list_set+0xbf>
    fd29:	ff 15 59 5b 05 00    	call   *0x55b59(%rip)        # 65888 <_DYNAMIC+0x328>
    fd2f:	48 8d 7c 24 28       	lea    0x28(%rsp),%rdi
    fd34:	e8 67 cb ff ff       	call   c8a0 <_RINvNtCscI6d9CVNmLh_4core3ptr9drop_glueINtNtCs40k4W9msRzi_5alloc3vec3VecyEECsk1kE6r7WBHz_11pythrust_rt.llvm.12450914586151300226>
    fd39:	ff 15 49 5b 05 00    	call   *0x55b49(%rip)        # 65888 <_DYNAMIC+0x328>
    fd3f:	48 8b 34 24          	mov    (%rsp),%rsi
    fd43:	48 85 f6             	test   %rsi,%rsi
    fd46:	75 06                	jne    fd4e <pt_list_set+0x21e>
    fd48:	ff 15 3a 5b 05 00    	call   *0x55b3a(%rip)        # 65888 <_DYNAMIC+0x328>
    fd4e:	48 8b 7c 24 08       	mov    0x8(%rsp),%rdi
    fd53:	48 c1 e6 03          	shl    $0x3,%rsi
    fd57:	ba 08 00 00 00       	mov    $0x8,%edx
    fd5c:	ff 15 c6 61 05 00    	call   *0x561c6(%rip)        # 65f28 <_DYNAMIC+0x9c8>
    fd62:	ff 15 20 5b 05 00    	call   *0x55b20(%rip)        # 65888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card815-run/baseline/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000bfe0 <py1_quicksort>:
    bfe0:	55                   	push   %rbp
    bfe1:	48 89 e5             	mov    %rsp,%rbp
    bfe4:	48 81 ec b0 00 00 00 	sub    $0xb0,%rsp
    bfeb:	48 89 9c 24 80 00 00 	mov    %rbx,0x80(%rsp)
    bff2:	00 
    bff3:	4c 89 a4 24 88 00 00 	mov    %r12,0x88(%rsp)
    bffa:	00 
    bffb:	4c 89 ac 24 90 00 00 	mov    %r13,0x90(%rsp)
    c002:	00 
    c003:	4c 89 b4 24 98 00 00 	mov    %r14,0x98(%rsp)
    c00a:	00 
    c00b:	4c 89 bc 24 a0 00 00 	mov    %r15,0xa0(%rsp)
    c012:	00 
    c013:	49 89 ff             	mov    %rdi,%r15
    c016:	48 89 74 24 38       	mov    %rsi,0x38(%rsp)
    c01b:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c020:	48 89 4c 24 58       	mov    %rcx,0x58(%rsp)
    c025:	48 8d 4c 24 10       	lea    0x10(%rsp),%rcx
    c02a:	48 89 4c 24 78       	mov    %rcx,0x78(%rsp)
    c02f:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c034:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c039:	48 3b d1             	cmp    %rcx,%rdx
    c03c:	0f 8d c0 03 00 00    	jge    c402 <py1_quicksort+0x422>
    c042:	48 89 d6             	mov    %rdx,%rsi
    c045:	48 03 f1             	add    %rcx,%rsi
    c048:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c04d:	48 89 4c 24 58       	mov    %rcx,0x58(%rsp)
    c052:	0f 90 c0             	seto   %al
    c055:	84 c0                	test   %al,%al
    c057:	0f 85 2b 08 00 00    	jne    c888 <py1_quicksort+0x8a8>
    c05d:	48 d1 fe             	sar    $1,%rsi
    c060:	48 8d 05 e9 3d 00 00 	lea    0x3de9(%rip),%rax        # fe50 <pt_seq_get>
    c067:	48 8b 54 24 78       	mov    0x78(%rsp),%rdx
    c06c:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c071:	ff d0                	call   *%rax
    c073:	48 89 44 24 70       	mov    %rax,0x70(%rsp)
    c078:	48 0f b6 44 24 10    	movzbq 0x10(%rsp),%rax
    c07e:	3c 01                	cmp    $0x1,%al
    c080:	0f 84 c1 07 00 00    	je     c847 <py1_quicksort+0x867>
    c086:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c08c:	48 8d 5c 24 28       	lea    0x28(%rsp),%rbx
    c091:	48 89 5c 24 68       	mov    %rbx,0x68(%rsp)
    c096:	4c 8d 64 24 30       	lea    0x30(%rsp),%r12
    c09b:	4c 89 64 24 60       	mov    %r12,0x60(%rsp)
    c0a0:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c0a5:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c0aa:	4d 89 f5             	mov    %r14,%r13
    c0ad:	45 84 f6             	test   %r14b,%r14b
    c0b0:	0f 84 7a 07 00 00    	je     c830 <py1_quicksort+0x850>
    c0b6:	45 84 ed             	test   %r13b,%r13b
    c0b9:	0f 84 5a 07 00 00    	je     c819 <py1_quicksort+0x839>
    c0bf:	48 8b 44 24 40       	mov    0x40(%rsp),%rax
    c0c4:	48 3b c1             	cmp    %rcx,%rax
    c0c7:	0f 8e 21 01 00 00    	jle    c1ee <py1_quicksort+0x20e>
    c0cd:	45 84 ed             	test   %r13b,%r13b
    c0d0:	0f 84 2f 04 00 00    	je     c505 <py1_quicksort+0x525>
    c0d6:	48 89 c8             	mov    %rcx,%rax
    c0d9:	48 2b c2             	sub    %rdx,%rax
    c0dc:	48 89 4c 24 48       	mov    %rcx,0x48(%rsp)
    c0e1:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
    c0e6:	0f 90 c1             	seto   %cl
    c0e9:	84 c9                	test   %cl,%cl
    c0eb:	0f 85 fd 03 00 00    	jne    c4ee <py1_quicksort+0x50e>
    c0f1:	45 84 f6             	test   %r14b,%r14b
    c0f4:	0f 84 dd 03 00 00    	je     c4d7 <py1_quicksort+0x4f7>
    c0fa:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c0ff:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c104:	48 2b ca             	sub    %rdx,%rcx
    c107:	0f 90 c2             	seto   %dl
    c10a:	84 d2                	test   %dl,%dl
    c10c:	0f 85 ae 03 00 00    	jne    c4c0 <py1_quicksort+0x4e0>
    c112:	48 3b c1             	cmp    %rcx,%rax
    c115:	0f 8c 72 00 00 00    	jl     c18d <py1_quicksort+0x1ad>
    c11b:	45 84 f6             	test   %r14b,%r14b
    c11e:	0f 84 40 03 00 00    	je     c464 <py1_quicksort+0x484>
    c124:	4d 89 fc             	mov    %r15,%r12
    c127:	4d 8b 74 24 08       	mov    0x8(%r12),%r14
    c12c:	49 81 fe e8 03 00 00 	cmp    $0x3e8,%r14
    c133:	0f 8d 14 03 00 00    	jge    c44d <py1_quicksort+0x46d>
    c139:	4d 8d 4e 01          	lea    0x1(%r14),%r9
    c13d:	4d 89 4c 24 08       	mov    %r9,0x8(%r12)
    c142:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c147:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c14c:	48 8b 74 24 38       	mov    0x38(%rsp),%rsi
    c151:	4c 89 e7             	mov    %r12,%rdi
    c154:	e8 87 fe ff ff       	call   bfe0 <py1_quicksort>
    c159:	4d 89 74 24 08       	mov    %r14,0x8(%r12)
    c15e:	49 0f b6 04 24       	movzbq (%r12),%rax
    c163:	84 c0                	test   %al,%al
    c165:	0f 85 97 02 00 00    	jne    c402 <py1_quicksort+0x422>
    c16b:	45 84 ed             	test   %r13b,%r13b
    c16e:	0f 84 c2 02 00 00    	je     c436 <py1_quicksort+0x456>
    c174:	48 8b 44 24 50       	mov    0x50(%rsp),%rax
    c179:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
    c17e:	48 8b 44 24 48       	mov    0x48(%rsp),%rax
    c183:	48 89 44 24 58       	mov    %rax,0x58(%rsp)
    c188:	e9 59 00 00 00       	jmp    c1e6 <py1_quicksort+0x206>
    c18d:	4d 89 fc             	mov    %r15,%r12
    c190:	45 84 ed             	test   %r13b,%r13b
    c193:	0f 84 10 03 00 00    	je     c4a9 <py1_quicksort+0x4c9>
    c199:	49 8b 5c 24 08       	mov    0x8(%r12),%rbx
    c19e:	48 81 fb e8 03 00 00 	cmp    $0x3e8,%rbx
    c1a5:	0f 8d e7 02 00 00    	jge    c492 <py1_quicksort+0x4b2>
    c1ab:	48 8d 43 01          	lea    0x1(%rbx),%rax
    c1af:	49 89 44 24 08       	mov    %rax,0x8(%r12)
    c1b4:	48 8b 4c 24 48       	mov    0x48(%rsp),%rcx
    c1b9:	48 8b 54 24 50       	mov    0x50(%rsp),%rdx
    c1be:	48 8b 74 24 38       	mov    0x38(%rsp),%rsi
    c1c3:	4c 89 e7             	mov    %r12,%rdi
    c1c6:	e8 15 fe ff ff       	call   bfe0 <py1_quicksort>
    c1cb:	49 89 5c 24 08       	mov    %rbx,0x8(%r12)
    c1d0:	49 0f b6 04 24       	movzbq (%r12),%rax
    c1d5:	84 c0                	test   %al,%al
    c1d7:	0f 85 25 02 00 00    	jne    c402 <py1_quicksort+0x422>
    c1dd:	45 84 f6             	test   %r14b,%r14b
    c1e0:	0f 84 95 02 00 00    	je     c47b <py1_quicksort+0x49b>
    c1e6:	4d 89 e7             	mov    %r12,%r15
    c1e9:	e9 41 fe ff ff       	jmp    c02f <py1_quicksort+0x4f>
    c1ee:	4d 89 fc             	mov    %r15,%r12
    c1f1:	48 89 4c 24 48       	mov    %rcx,0x48(%rsp)
    c1f6:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
    c1fb:	48 8d 5c 24 18       	lea    0x18(%rsp),%rbx
    c200:	45 84 f6             	test   %r14b,%r14b
    c203:	0f 84 f9 05 00 00    	je     c802 <py1_quicksort+0x822>
    c209:	48 8d 05 40 3c 00 00 	lea    0x3c40(%rip),%rax        # fe50 <pt_seq_get>
    c210:	48 89 da             	mov    %rbx,%rdx
    c213:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c218:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c21d:	ff d0                	call   *%rax
    c21f:	48 0f b6 4c 24 18    	movzbq 0x18(%rsp),%rcx
    c225:	40 80 f9 01          	rex cmp $0x1,%cl
    c229:	0f 84 92 05 00 00    	je     c7c1 <py1_quicksort+0x7e1>
    c22f:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
    c234:	48 3b c1             	cmp    %rcx,%rax
    c237:	0f 8c 8e 01 00 00    	jl     c3cb <py1_quicksort+0x3eb>
    c23d:	48 8d 5c 24 20       	lea    0x20(%rsp),%rbx
    c242:	45 84 ed             	test   %r13b,%r13b
    c245:	0f 84 31 05 00 00    	je     c77c <py1_quicksort+0x79c>
    c24b:	4c 8d 05 fe 3b 00 00 	lea    0x3bfe(%rip),%r8        # fe50 <pt_seq_get>
    c252:	4c 8b 7c 24 48       	mov    0x48(%rsp),%r15
    c257:	48 89 da             	mov    %rbx,%rdx
    c25a:	4c 89 fe             	mov    %r15,%rsi
    c25d:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c262:	41 ff d0             	call   *%r8
    c265:	48 0f b6 74 24 20    	movzbq 0x20(%rsp),%rsi
    c26b:	40 80 fe 01          	cmp    $0x1,%sil
    c26f:	0f 84 c6 04 00 00    	je     c73b <py1_quicksort+0x75b>
    c275:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
    c27a:	48 3b c1             	cmp    %rcx,%rax
    c27d:	0f 8f 20 01 00 00    	jg     c3a3 <py1_quicksort+0x3c3>
    c283:	45 84 f6             	test   %r14b,%r14b
    c286:	0f 84 6a 04 00 00    	je     c6f6 <py1_quicksort+0x716>
    c28c:	45 84 ed             	test   %r13b,%r13b
    c28f:	0f 84 4a 04 00 00    	je     c6df <py1_quicksort+0x6ff>
    c295:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c29a:	49 3b d7             	cmp    %r15,%rdx
    c29d:	0f 8f f0 00 00 00    	jg     c393 <py1_quicksort+0x3b3>
    c2a3:	45 84 f6             	test   %r14b,%r14b
    c2a6:	0f 84 1c 04 00 00    	je     c6c8 <py1_quicksort+0x6e8>
    c2ac:	48 8d 05 9d 3b 00 00 	lea    0x3b9d(%rip),%rax        # fe50 <pt_seq_get>
    c2b3:	48 8b 54 24 68       	mov    0x68(%rsp),%rdx
    c2b8:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c2bd:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c2c2:	ff d0                	call   *%rax
    c2c4:	48 89 c3             	mov    %rax,%rbx
    c2c7:	48 0f b6 44 24 28    	movzbq 0x28(%rsp),%rax
    c2cd:	3c 01                	cmp    $0x1,%al
    c2cf:	0f 84 b2 03 00 00    	je     c687 <py1_quicksort+0x6a7>
    c2d5:	45 84 ed             	test   %r13b,%r13b
    c2d8:	0f 84 91 03 00 00    	je     c66f <py1_quicksort+0x68f>
    c2de:	48 8d 05 6b 3b 00 00 	lea    0x3b6b(%rip),%rax        # fe50 <pt_seq_get>
    c2e5:	48 8b 54 24 60       	mov    0x60(%rsp),%rdx
    c2ea:	4c 89 fe             	mov    %r15,%rsi
    c2ed:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c2f2:	ff d0                	call   *%rax
    c2f4:	48 0f b6 4c 24 30    	movzbq 0x30(%rsp),%rcx
    c2fa:	40 80 f9 01          	rex cmp $0x1,%cl
    c2fe:	0f 84 29 03 00 00    	je     c62d <py1_quicksort+0x64d>
    c304:	45 84 f6             	test   %r14b,%r14b
    c307:	0f 84 08 03 00 00    	je     c615 <py1_quicksort+0x635>
    c30d:	48 8d 0d 1c 38 00 00 	lea    0x381c(%rip),%rcx        # fb30 <pt_list_set>
    c314:	48 89 c2             	mov    %rax,%rdx
    c317:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c31c:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c321:	ff d1                	call   *%rcx
    c323:	3c 01                	cmp    $0x1,%al
    c325:	0f 84 a5 02 00 00    	je     c5d0 <py1_quicksort+0x5f0>
    c32b:	45 84 ed             	test   %r13b,%r13b
    c32e:	0f 84 85 02 00 00    	je     c5b9 <py1_quicksort+0x5d9>
    c334:	48 8d 05 f5 37 00 00 	lea    0x37f5(%rip),%rax        # fb30 <pt_list_set>
    c33b:	48 89 da             	mov    %rbx,%rdx
    c33e:	4c 89 fe             	mov    %r15,%rsi
    c341:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c346:	ff d0                	call   *%rax
    c348:	3c 01                	cmp    $0x1,%al
    c34a:	0f 84 28 02 00 00    	je     c578 <py1_quicksort+0x598>
    c350:	45 84 f6             	test   %r14b,%r14b
    c353:	0f 84 08 02 00 00    	je     c561 <py1_quicksort+0x581>
    c359:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c35e:	48 83 c2 01          	add    $0x1,%rdx
    c362:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c367:	0f 90 c0             	seto   %al
    c36a:	84 c0                	test   %al,%al
    c36c:	0f 85 d8 01 00 00    	jne    c54a <py1_quicksort+0x56a>
    c372:	45 84 ed             	test   %r13b,%r13b
    c375:	0f 84 b8 01 00 00    	je     c533 <py1_quicksort+0x553>
    c37b:	49 83 ef 01          	sub    $0x1,%r15
    c37f:	0f 90 c0             	seto   %al
    c382:	84 c0                	test   %al,%al
    c384:	0f 85 92 01 00 00    	jne    c51c <py1_quicksort+0x53c>
    c38a:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c390:	4d 89 f5             	mov    %r14,%r13
    c393:	48 8b 54 24 50       	mov    0x50(%rsp),%rdx
    c398:	4c 89 f9             	mov    %r15,%rcx
    c39b:	4d 89 e7             	mov    %r12,%r15
    c39e:	e9 0a fd ff ff       	jmp    c0ad <py1_quicksort+0xcd>
    c3a3:	45 84 ed             	test   %r13b,%r13b
    c3a6:	0f 84 78 03 00 00    	je     c724 <py1_quicksort+0x744>
    c3ac:	49 83 ef 01          	sub    $0x1,%r15
    c3b0:	0f 90 c0             	seto   %al
    c3b3:	84 c0                	test   %al,%al
    c3b5:	0f 85 52 03 00 00    	jne    c70d <py1_quicksort+0x72d>
    c3bb:	41 bd 01 00 00 00    	mov    $0x1,%r13d
    c3c1:	4c 89 7c 24 48       	mov    %r15,0x48(%rsp)
    c3c6:	e9 77 fe ff ff       	jmp    c242 <py1_quicksort+0x262>
    c3cb:	4c 8b 7c 24 48       	mov    0x48(%rsp),%r15
    c3d0:	45 84 f6             	test   %r14b,%r14b
    c3d3:	0f 84 d1 03 00 00    	je     c7aa <py1_quicksort+0x7ca>
    c3d9:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c3de:	48 83 c2 01          	add    $0x1,%rdx
    c3e2:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c3e7:	0f 90 c0             	seto   %al
    c3ea:	84 c0                	test   %al,%al
    c3ec:	0f 85 a1 03 00 00    	jne    c793 <py1_quicksort+0x7b3>
    c3f2:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c3f8:	4c 89 7c 24 48       	mov    %r15,0x48(%rsp)
    c3fd:	e9 fe fd ff ff       	jmp    c200 <py1_quicksort+0x220>
    c402:	48 8b 9c 24 80 00 00 	mov    0x80(%rsp),%rbx
    c409:	00 
    c40a:	4c 8b a4 24 88 00 00 	mov    0x88(%rsp),%r12
    c411:	00 
    c412:	4c 8b ac 24 90 00 00 	mov    0x90(%rsp),%r13
    c419:	00 
    c41a:	4c 8b b4 24 98 00 00 	mov    0x98(%rsp),%r14
    c421:	00 
    c422:	4c 8b bc 24 a0 00 00 	mov    0xa0(%rsp),%r15
    c429:	00 
    c42a:	48 81 c4 b0 00 00 00 	add    $0xb0,%rsp
    c431:	48 89 ec             	mov    %rbp,%rsp
    c434:	5d                   	pop    %rbp
    c435:	c3                   	ret
    c436:	48 8d 3d 84 51 04 00 	lea    0x45184(%rip),%rdi        # 515c1 <.Ldata49>
    c43d:	be c6 00 00 00       	mov    $0xc6,%esi
    c442:	48 8d 05 b7 64 00 00 	lea    0x64b7(%rip),%rax        # 12900 <pt_fail>
    c449:	ff d0                	call   *%rax
    c44b:	0f 0b                	ud2
    c44d:	48 8d 3d b6 50 04 00 	lea    0x450b6(%rip),%rdi        # 5150a <.Ldata48>
    c454:	be b7 00 00 00       	mov    $0xb7,%esi
    c459:	48 8d 05 a0 64 00 00 	lea    0x64a0(%rip),%rax        # 12900 <pt_fail>
    c460:	ff d0                	call   *%rax
    c462:	0f 0b                	ud2
    c464:	48 8d 3d c2 4f 04 00 	lea    0x44fc2(%rip),%rdi        # 5142d <.Ldata47>
    c46b:	be dd 00 00 00       	mov    $0xdd,%esi
    c470:	48 8d 05 89 64 00 00 	lea    0x6489(%rip),%rax        # 12900 <pt_fail>
    c477:	ff d0                	call   *%rax
    c479:	0f 0b                	ud2
    c47b:	48 8d 3d e5 4e 04 00 	lea    0x44ee5(%rip),%rdi        # 51367 <.Ldata46>
    c482:	be c6 00 00 00       	mov    $0xc6,%esi
    c487:	48 8d 05 72 64 00 00 	lea    0x6472(%rip),%rax        # 12900 <pt_fail>
    c48e:	ff d0                	call   *%rax
    c490:	0f 0b                	ud2
    c492:	48 8d 3d 17 4e 04 00 	lea    0x44e17(%rip),%rdi        # 512b0 <.Ldata45>
    c499:	be b7 00 00 00       	mov    $0xb7,%esi
    c49e:	48 8d 05 5b 64 00 00 	lea    0x645b(%rip),%rax        # 12900 <pt_fail>
    c4a5:	ff d0                	call   *%rax
    c4a7:	0f 0b                	ud2
    c4a9:	48 8d 3d 1f 4d 04 00 	lea    0x44d1f(%rip),%rdi        # 511cf <.Ldata44>
    c4b0:	be e1 00 00 00       	mov    $0xe1,%esi
    c4b5:	48 8d 05 44 64 00 00 	lea    0x6444(%rip),%rax        # 12900 <pt_fail>
    c4bc:	ff d0                	call   *%rax
    c4be:	0f 0b                	ud2
    c4c0:	48 8d 3d 6d 4c 04 00 	lea    0x44c6d(%rip),%rdi        # 51134 <.Ldata43>
    c4c7:	be 9b 00 00 00       	mov    $0x9b,%esi
    c4cc:	48 8d 05 2d 64 00 00 	lea    0x642d(%rip),%rax        # 12900 <pt_fail>
    c4d3:	ff d0                	call   *%rax
    c4d5:	0f 0b                	ud2
    c4d7:	48 8d 3d 7f 4b 04 00 	lea    0x44b7f(%rip),%rdi        # 5105d <.Ldata42>
    c4de:	be d7 00 00 00       	mov    $0xd7,%esi
    c4e3:	48 8d 05 16 64 00 00 	lea    0x6416(%rip),%rax        # 12900 <pt_fail>
    c4ea:	ff d0                	call   *%rax
    c4ec:	0f 0b                	ud2
    c4ee:	48 8d 3d d6 4a 04 00 	lea    0x44ad6(%rip),%rdi        # 50fcb <.Ldata41>
    c4f5:	be 92 00 00 00       	mov    $0x92,%esi
    c4fa:	48 8d 05 ff 63 00 00 	lea    0x63ff(%rip),%rax        # 12900 <pt_fail>
    c501:	ff d0                	call   *%rax
    c503:	0f 0b                	ud2
    c505:	48 8d 3d f6 49 04 00 	lea    0x449f6(%rip),%rdi        # 50f02 <.Ldata40>
    c50c:	be c9 00 00 00       	mov    $0xc9,%esi
    c511:	48 8d 05 e8 63 00 00 	lea    0x63e8(%rip),%rax        # 12900 <pt_fail>
    c518:	ff d0                	call   *%rax
    c51a:	0f 0b                	ud2
    c51c:	48 8d 3d 4d 49 04 00 	lea    0x4494d(%rip),%rdi        # 50e70 <.Ldata3f>
    c523:	be 92 00 00 00       	mov    $0x92,%esi
    c528:	48 8d 05 d1 63 00 00 	lea    0x63d1(%rip),%rax        # 12900 <pt_fail>
    c52f:	ff d0                	call   *%rax
    c531:	0f 0b                	ud2
    c533:	48 8d 3d 6d 48 04 00 	lea    0x4486d(%rip),%rdi        # 50da7 <.Ldata3e>
    c53a:	be c9 00 00 00       	mov    $0xc9,%esi
    c53f:	48 8d 05 ba 63 00 00 	lea    0x63ba(%rip),%rax        # 12900 <pt_fail>
    c546:	ff d0                	call   *%rax
    c548:	0f 0b                	ud2
    c54a:	48 8d 3d c4 47 04 00 	lea    0x447c4(%rip),%rdi        # 50d15 <.Ldata3d>
    c551:	be 92 00 00 00       	mov    $0x92,%esi
    c556:	48 8d 05 a3 63 00 00 	lea    0x63a3(%rip),%rax        # 12900 <pt_fail>
    c55d:	ff d0                	call   *%rax
    c55f:	0f 0b                	ud2
    c561:	48 8d 3d e4 46 04 00 	lea    0x446e4(%rip),%rdi        # 50c4c <.Ldata3c>
    c568:	be c9 00 00 00       	mov    $0xc9,%esi
    c56d:	48 8d 05 8c 63 00 00 	lea    0x638c(%rip),%rax        # 12900 <pt_fail>
    c574:	ff d0                	call   *%rax
    c576:	0f 0b                	ud2
    c578:	b8 01 00 00 00       	mov    $0x1,%eax
    c57d:	48 8d 35 15 44 04 00 	lea    0x44415(%rip),%rsi        # 50999 <.Ldata35>
    c584:	ba 22 00 00 00       	mov    $0x22,%edx
    c589:	48 8d 0d 47 46 04 00 	lea    0x44647(%rip),%rcx        # 50bd7 <.Ldata3a>
    c590:	41 b8 40 00 00 00    	mov    $0x40,%r8d
    c596:	4c 8d 0d 7a 46 04 00 	lea    0x4467a(%rip),%r9        # 50c17 <.Ldata3b>
    c59d:	41 ba 35 00 00 00    	mov    $0x35,%r10d
    c5a3:	48 0f b6 f8          	movzbq %al,%rdi
    c5a7:	4c 89 14 24          	mov    %r10,(%rsp)
    c5ab:	48 8d 05 7e 6c 00 00 	lea    0x6c7e(%rip),%rax        # 13230 <pt_raise_msg>
    c5b2:	ff d0                	call   *%rax
    c5b4:	e9 49 fe ff ff       	jmp    c402 <py1_quicksort+0x422>
    c5b9:	48 8d 3d 48 45 04 00 	lea    0x44548(%rip),%rdi        # 50b08 <.Ldata39>
    c5c0:	be cf 00 00 00       	mov    $0xcf,%esi
    c5c5:	48 8d 05 34 63 00 00 	lea    0x6334(%rip),%rax        # 12900 <pt_fail>
    c5cc:	ff d0                	call   *%rax
    c5ce:	0f 0b                	ud2
    c5d0:	b9 01 00 00 00       	mov    $0x1,%ecx
    c5d5:	48 8d 35 bd 43 04 00 	lea    0x443bd(%rip),%rsi        # 50999 <.Ldata35>
    c5dc:	ba 22 00 00 00       	mov    $0x22,%edx
    c5e1:	4c 8d 15 d3 43 04 00 	lea    0x443d3(%rip),%r10        # 509bb <.Ldata36>
    c5e8:	41 b8 40 00 00 00    	mov    $0x40,%r8d
    c5ee:	4c 8d 0d 06 44 04 00 	lea    0x44406(%rip),%r9        # 509fb <.Ldata37>
    c5f5:	41 bb 39 00 00 00    	mov    $0x39,%r11d
    c5fb:	48 0f b6 f9          	movzbq %cl,%rdi
    c5ff:	4c 89 1c 24          	mov    %r11,(%rsp)
    c603:	4c 8d 1d 26 6c 00 00 	lea    0x6c26(%rip),%r11        # 13230 <pt_raise_msg>
    c60a:	4c 89 d1             	mov    %r10,%rcx
    c60d:	41 ff d3             	call   *%r11
    c610:	e9 ed fd ff ff       	jmp    c402 <py1_quicksort+0x422>
    c615:	48 8d 3d aa 42 04 00 	lea    0x442aa(%rip),%rdi        # 508c6 <.Ldata34>
    c61c:	be d3 00 00 00       	mov    $0xd3,%esi
    c621:	4c 8d 05 d8 62 00 00 	lea    0x62d8(%rip),%r8        # 12900 <pt_fail>
    c628:	41 ff d0             	call   *%r8
    c62b:	0f 0b                	ud2
    c62d:	bf 01 00 00 00       	mov    $0x1,%edi
    c632:	48 8d 35 fd 2e 04 00 	lea    0x42efd(%rip),%rsi        # 4f536 <.Ldatac>
    c639:	ba 17 00 00 00       	mov    $0x17,%edx
    c63e:	48 8d 0d 00 42 04 00 	lea    0x44200(%rip),%rcx        # 50845 <.Ldata32>
    c645:	41 b8 40 00 00 00    	mov    $0x40,%r8d
    c64b:	4c 8d 0d 33 42 04 00 	lea    0x44233(%rip),%r9        # 50885 <.Ldata33>
    c652:	41 ba 41 00 00 00    	mov    $0x41,%r10d
    c658:	48 0f b6 ff          	movzbq %dil,%rdi
    c65c:	4c 89 14 24          	mov    %r10,(%rsp)
    c660:	4c 8d 15 c9 6b 00 00 	lea    0x6bc9(%rip),%r10        # 13230 <pt_raise_msg>
    c667:	41 ff d2             	call   *%r10
    c66a:	e9 93 fd ff ff       	jmp    c402 <py1_quicksort+0x422>
    c66f:	48 8d 3d f4 40 04 00 	lea    0x440f4(%rip),%rdi        # 5076a <.Ldata31>
    c676:	be db 00 00 00       	mov    $0xdb,%esi
    c67b:	4c 8d 1d 7e 62 00 00 	lea    0x627e(%rip),%r11        # 12900 <pt_fail>
    c682:	41 ff d3             	call   *%r11
    c685:	0f 0b                	ud2
    c687:	b8 01 00 00 00       	mov    $0x1,%eax
    c68c:	48 8d 35 a3 2e 04 00 	lea    0x42ea3(%rip),%rsi        # 4f536 <.Ldatac>
    c693:	ba 17 00 00 00       	mov    $0x17,%edx
    c698:	48 8d 0d 52 40 04 00 	lea    0x44052(%rip),%rcx        # 506f1 <.Ldata2f>
    c69f:	41 b8 40 00 00 00    	mov    $0x40,%r8d
    c6a5:	4c 8d 0d 85 40 04 00 	lea    0x44085(%rip),%r9        # 50731 <.Ldata30>
    c6ac:	41 ba 39 00 00 00    	mov    $0x39,%r10d
    c6b2:	48 0f b6 f8          	movzbq %al,%rdi
    c6b6:	4c 89 14 24          	mov    %r10,(%rsp)
    c6ba:	48 8d 05 6f 6b 00 00 	lea    0x6b6f(%rip),%rax        # 13230 <pt_raise_msg>
    c6c1:	ff d0                	call   *%rax
    c6c3:	e9 3a fd ff ff       	jmp    c402 <py1_quicksort+0x422>
    c6c8:	48 8d 3d 4f 3f 04 00 	lea    0x43f4f(%rip),%rdi        # 5061e <.Ldata2e>
    c6cf:	be d3 00 00 00       	mov    $0xd3,%esi
    c6d4:	48 8d 05 25 62 00 00 	lea    0x6225(%rip),%rax        # 12900 <pt_fail>
    c6db:	ff d0                	call   *%rax
    c6dd:	0f 0b                	ud2
    c6df:	48 8d 3d 6b 3e 04 00 	lea    0x43e6b(%rip),%rdi        # 50551 <.Ldata2d>
    c6e6:	be cd 00 00 00       	mov    $0xcd,%esi
    c6eb:	48 8d 05 0e 62 00 00 	lea    0x620e(%rip),%rax        # 12900 <pt_fail>
    c6f2:	ff d0                	call   *%rax
    c6f4:	0f 0b                	ud2
    c6f6:	48 8d 3d 8c 3d 04 00 	lea    0x43d8c(%rip),%rdi        # 50489 <.Ldata2c>
    c6fd:	be c8 00 00 00       	mov    $0xc8,%esi
    c702:	48 8d 05 f7 61 00 00 	lea    0x61f7(%rip),%rax        # 12900 <pt_fail>
    c709:	ff d0                	call   *%rax
    c70b:	0f 0b                	ud2
    c70d:	48 8d 3d e3 3c 04 00 	lea    0x43ce3(%rip),%rdi        # 503f7 <.Ldata2b>
    c714:	be 92 00 00 00       	mov    $0x92,%esi
    c719:	48 8d 05 e0 61 00 00 	lea    0x61e0(%rip),%rax        # 12900 <pt_fail>
    c720:	ff d0                	call   *%rax
    c722:	0f 0b                	ud2
    c724:	48 8d 3d 03 3c 04 00 	lea    0x43c03(%rip),%rdi        # 5032e <.Ldata2a>
    c72b:	be c9 00 00 00       	mov    $0xc9,%esi
    c730:	48 8d 05 c9 61 00 00 	lea    0x61c9(%rip),%rax        # 12900 <pt_fail>
    c737:	ff d0                	call   *%rax
    c739:	0f 0b                	ud2
    c73b:	b8 01 00 00 00       	mov    $0x1,%eax
    c740:	48 8d 35 ef 2d 04 00 	lea    0x42def(%rip),%rsi        # 4f536 <.Ldatac>
    c747:	ba 17 00 00 00       	mov    $0x17,%edx
    c74c:	48 8d 0d 78 3a 04 00 	lea    0x43a78(%rip),%rcx        # 501cb <.Ldata27>
    c753:	41 b8 40 00 00 00    	mov    $0x40,%r8d
    c759:	4c 8d 0d ab 3a 04 00 	lea    0x43aab(%rip),%r9        # 5020b <.Ldata28>
    c760:	41 ba 3e 00 00 00    	mov    $0x3e,%r10d
    c766:	48 0f b6 f8          	movzbq %al,%rdi
    c76a:	4c 89 14 24          	mov    %r10,(%rsp)
    c76e:	48 8d 05 bb 6a 00 00 	lea    0x6abb(%rip),%rax        # 13230 <pt_raise_msg>
    c775:	ff d0                	call   *%rax
    c777:	e9 86 fc ff ff       	jmp    c402 <py1_quicksort+0x422>
    c77c:	48 8d 3d 70 39 04 00 	lea    0x43970(%rip),%rdi        # 500f3 <.Ldata26>
    c783:	be d8 00 00 00       	mov    $0xd8,%esi
    c788:	48 8d 05 71 61 00 00 	lea    0x6171(%rip),%rax        # 12900 <pt_fail>
    c78f:	ff d0                	call   *%rax
    c791:	0f 0b                	ud2
    c793:	48 8d 3d c8 38 04 00 	lea    0x438c8(%rip),%rdi        # 50062 <.Ldata25>
    c79a:	be 91 00 00 00       	mov    $0x91,%esi
    c79f:	48 8d 05 5a 61 00 00 	lea    0x615a(%rip),%rax        # 12900 <pt_fail>
    c7a6:	ff d0                	call   *%rax
    c7a8:	0f 0b                	ud2
    c7aa:	48 8d 3d e9 37 04 00 	lea    0x437e9(%rip),%rdi        # 4ff9a <.Ldata24>
    c7b1:	be c8 00 00 00       	mov    $0xc8,%esi
    c7b6:	48 8d 05 43 61 00 00 	lea    0x6143(%rip),%rax        # 12900 <pt_fail>
    c7bd:	ff d0                	call   *%rax
    c7bf:	0f 0b                	ud2
    c7c1:	b8 01 00 00 00       	mov    $0x1,%eax
    c7c6:	48 8d 35 69 2d 04 00 	lea    0x42d69(%rip),%rsi        # 4f536 <.Ldatac>
    c7cd:	ba 17 00 00 00       	mov    $0x17,%edx
    c7d2:	48 8d 0d 60 36 04 00 	lea    0x43660(%rip),%rcx        # 4fe39 <.Ldata21>
    c7d9:	41 b8 3f 00 00 00    	mov    $0x3f,%r8d
    c7df:	4c 8d 0d 92 36 04 00 	lea    0x43692(%rip),%r9        # 4fe78 <.Ldata22>
    c7e6:	41 ba 3e 00 00 00    	mov    $0x3e,%r10d
    c7ec:	48 0f b6 f8          	movzbq %al,%rdi
    c7f0:	4c 89 14 24          	mov    %r10,(%rsp)
    c7f4:	48 8d 05 35 6a 00 00 	lea    0x6a35(%rip),%rax        # 13230 <pt_raise_msg>
    c7fb:	ff d0                	call   *%rax
    c7fd:	e9 00 fc ff ff       	jmp    c402 <py1_quicksort+0x422>
    c802:	48 8d 3d 59 35 04 00 	lea    0x43559(%rip),%rdi        # 4fd62 <.Ldata20>
    c809:	be d7 00 00 00       	mov    $0xd7,%esi
    c80e:	48 8d 05 eb 60 00 00 	lea    0x60eb(%rip),%rax        # 12900 <pt_fail>
    c815:	ff d0                	call   *%rax
    c817:	0f 0b                	ud2
    c819:	48 8d 3d 78 34 04 00 	lea    0x43478(%rip),%rdi        # 4fc98 <.Ldata1f>
    c820:	be ca 00 00 00       	mov    $0xca,%esi
    c825:	48 8d 05 d4 60 00 00 	lea    0x60d4(%rip),%rax        # 12900 <pt_fail>
    c82c:	ff d0                	call   *%rax
    c82e:	0f 0b                	ud2
    c830:	48 8d 3d 9c 33 04 00 	lea    0x4339c(%rip),%rdi        # 4fbd3 <.Ldata1e>
    c837:	be c5 00 00 00       	mov    $0xc5,%esi
    c83c:	48 8d 05 bd 60 00 00 	lea    0x60bd(%rip),%rax        # 12900 <pt_fail>
    c843:	ff d0                	call   *%rax
    c845:	0f 0b                	ud2
    c847:	b8 01 00 00 00       	mov    $0x1,%eax
    c84c:	48 8d 35 e3 2c 04 00 	lea    0x42ce3(%rip),%rsi        # 4f536 <.Ldatac>
    c853:	ba 17 00 00 00       	mov    $0x17,%edx
    c858:	48 8d 0d ea 32 04 00 	lea    0x432ea(%rip),%rcx        # 4fb49 <.Ldata1c>
    c85f:	41 b8 3f 00 00 00    	mov    $0x3f,%r8d
    c865:	4c 8d 0d 1c 33 04 00 	lea    0x4331c(%rip),%r9        # 4fb88 <.Ldata1d>
    c86c:	41 ba 4b 00 00 00    	mov    $0x4b,%r10d
    c872:	48 0f b6 f8          	movzbq %al,%rdi
    c876:	4c 89 14 24          	mov    %r10,(%rsp)
    c87a:	48 8d 05 af 69 00 00 	lea    0x69af(%rip),%rax        # 13230 <pt_raise_msg>
    c881:	ff d0                	call   *%rax
    c883:	e9 7a fb ff ff       	jmp    c402 <py1_quicksort+0x422>
    c888:	48 8d 3d 17 32 04 00 	lea    0x43217(%rip),%rdi        # 4faa6 <.Ldata1b>
    c88f:	be a3 00 00 00       	mov    $0xa3,%esi
    c894:	48 8d 05 65 60 00 00 	lea    0x6065(%rip),%rax        # 12900 <pt_fail>
    c89b:	ff d0                	call   *%rax
    c89d:	0f 0b                	ud2

Disassembly of section .fini:
