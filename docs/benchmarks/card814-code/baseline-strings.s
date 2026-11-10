
/workspace/scratch/2ce9dd46a989/card814-base/strings:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000bad0 <pythrust_main>:
    bad0:	55                   	push   %rbp
    bad1:	48 89 e5             	mov    %rsp,%rbp
    bad4:	48 81 ec 80 00 00 00 	sub    $0x80,%rsp
    badb:	48 89 5c 24 50       	mov    %rbx,0x50(%rsp)
    bae0:	4c 89 64 24 58       	mov    %r12,0x58(%rsp)
    bae5:	4c 89 6c 24 60       	mov    %r13,0x60(%rsp)
    baea:	4c 89 74 24 68       	mov    %r14,0x68(%rsp)
    baef:	4c 89 7c 24 70       	mov    %r15,0x70(%rsp)
    baf4:	48 8d 3d 05 15 04 00 	lea    0x41505(%rip),%rdi        # 4d000 <.Ldata1>
    bafb:	48 33 db             	xor    %rbx,%rbx
    bafe:	48 8d 05 9b 48 00 00 	lea    0x489b(%rip),%rax        # 103a0 <pt_str_new>
    bb05:	48 89 de             	mov    %rbx,%rsi
    bb08:	ff d0                	call   *%rax
    bb0a:	4c 8d 25 ef 14 04 00 	lea    0x414ef(%rip),%r12        # 4d000 <.Ldata1>
    bb11:	4c 8d 2d e9 14 04 00 	lea    0x414e9(%rip),%r13        # 4d001 <.Ldata2>
    bb18:	49 89 c7             	mov    %rax,%r15
    bb1b:	48 89 5c 24 10       	mov    %rbx,0x10(%rsp)
    bb20:	48 89 5c 24 18       	mov    %rbx,0x18(%rsp)
    bb25:	48 81 fb 40 42 0f 00 	cmp    $0xf4240,%rbx
    bb2c:	0f 8d 40 02 00 00    	jge    bd72 <pythrust_main+0x2a2>
    bb32:	41 be 01 00 00 00    	mov    $0x1,%r14d
    bb38:	48 8d 05 61 48 00 00 	lea    0x4861(%rip),%rax        # 103a0 <pt_str_new>
    bb3f:	4c 89 f6             	mov    %r14,%rsi
    bb42:	4c 89 e7             	mov    %r12,%rdi
    bb45:	ff d0                	call   *%rax
    bb47:	48 89 44 24 48       	mov    %rax,0x48(%rsp)
    bb4c:	48 8d 05 4d 3f 00 00 	lea    0x3f4d(%rip),%rax        # faa0 <pt_str_from_int>
    bb53:	48 89 df             	mov    %rbx,%rdi
    bb56:	ff d0                	call   *%rax
    bb58:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
    bb5d:	48 8d 05 7c 3d 00 00 	lea    0x3d7c(%rip),%rax        # f8e0 <pt_str_concat>
    bb64:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    bb69:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bb6e:	ff d0                	call   *%rax
    bb70:	48 89 44 24 38       	mov    %rax,0x38(%rsp)
    bb75:	48 8d 05 24 48 00 00 	lea    0x4824(%rip),%rax        # 103a0 <pt_str_new>
    bb7c:	4c 89 f6             	mov    %r14,%rsi
    bb7f:	4c 89 ef             	mov    %r13,%rdi
    bb82:	ff d0                	call   *%rax
    bb84:	48 89 44 24 30       	mov    %rax,0x30(%rsp)
    bb89:	48 8d 0d 50 3d 00 00 	lea    0x3d50(%rip),%rcx        # f8e0 <pt_str_concat>
    bb90:	48 8b 74 24 30       	mov    0x30(%rsp),%rsi
    bb95:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    bb9a:	ff d1                	call   *%rcx
    bb9c:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
    bba1:	48 89 d8             	mov    %rbx,%rax
    bba4:	48 f7 2d f5 02 00 00 	imulq  0x2f5(%rip)        # bea0 <pythrust_main+0x3d0>
    bbab:	48 8d 04 1a          	lea    (%rdx,%rbx,1),%rax
    bbaf:	48 c1 f8 06          	sar    $0x6,%rax
    bbb3:	48 89 c1             	mov    %rax,%rcx
    bbb6:	48 c1 e9 3f          	shr    $0x3f,%rcx
    bbba:	48 03 c1             	add    %rcx,%rax
    bbbd:	48 6b c8 61          	imul   $0x61,%rax,%rcx
    bbc1:	48 89 d8             	mov    %rbx,%rax
    bbc4:	48 2b c1             	sub    %rcx,%rax
    bbc7:	48 3b d9             	cmp    %rcx,%rbx
    bbca:	0f 95 c1             	setne  %cl
    bbcd:	48 89 c6             	mov    %rax,%rsi
    bbd0:	48 83 f6 61          	xor    $0x61,%rsi
    bbd4:	48 33 d2             	xor    %rdx,%rdx
    bbd7:	48 c1 ee 3f          	shr    $0x3f,%rsi
    bbdb:	40 84 ce             	test   %cl,%sil
    bbde:	48 0f 45 15 c2 02 00 	cmovne 0x2c2(%rip),%rdx        # bea8 <pythrust_main+0x3d8>
    bbe5:	00 
    bbe6:	48 8d 3c 10          	lea    (%rax,%rdx,1),%rdi
    bbea:	48 8d 05 af 3e 00 00 	lea    0x3eaf(%rip),%rax        # faa0 <pt_str_from_int>
    bbf1:	ff d0                	call   *%rax
    bbf3:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
    bbf8:	48 8d 0d e1 3c 00 00 	lea    0x3ce1(%rip),%rcx        # f8e0 <pt_str_concat>
    bbff:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
    bc04:	48 8b 7c 24 20       	mov    0x20(%rsp),%rdi
    bc09:	ff d1                	call   *%rcx
    bc0b:	49 89 c6             	mov    %rax,%r14
    bc0e:	48 8d 05 0b 3e 00 00 	lea    0x3e0b(%rip),%rax        # fa20 <pt_str_dec>
    bc15:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
    bc1a:	ff d0                	call   *%rax
    bc1c:	48 8d 05 fd 3d 00 00 	lea    0x3dfd(%rip),%rax        # fa20 <pt_str_dec>
    bc23:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bc28:	ff d0                	call   *%rax
    bc2a:	48 8d 05 ef 3d 00 00 	lea    0x3def(%rip),%rax        # fa20 <pt_str_dec>
    bc31:	48 8b 7c 24 40       	mov    0x40(%rsp),%rdi
    bc36:	ff d0                	call   *%rax
    bc38:	48 8d 05 e1 3d 00 00 	lea    0x3de1(%rip),%rax        # fa20 <pt_str_dec>
    bc3f:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    bc44:	ff d0                	call   *%rax
    bc46:	48 8d 05 d3 3d 00 00 	lea    0x3dd3(%rip),%rax        # fa20 <pt_str_dec>
    bc4d:	48 8b 7c 24 30       	mov    0x30(%rsp),%rdi
    bc52:	ff d0                	call   *%rax
    bc54:	48 8d 0d c5 3d 00 00 	lea    0x3dc5(%rip),%rcx        # fa20 <pt_str_dec>
    bc5b:	48 8b 7c 24 20       	mov    0x20(%rsp),%rdi
    bc60:	ff d1                	call   *%rcx
    bc62:	48 8d 15 b7 3d 00 00 	lea    0x3db7(%rip),%rdx        # fa20 <pt_str_dec>
    bc69:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
    bc6e:	ff d2                	call   *%rdx
    bc70:	48 8d 35 09 47 00 00 	lea    0x4709(%rip),%rsi        # 10380 <pt_str_len>
    bc77:	4c 89 f7             	mov    %r14,%rdi
    bc7a:	ff d6                	call   *%rsi
    bc7c:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
    bc81:	48 03 f8             	add    %rax,%rdi
    bc84:	48 89 7c 24 10       	mov    %rdi,0x10(%rsp)
    bc89:	40 0f 90 c6          	seto   %sil
    bc8d:	40 84 f6             	test   %sil,%sil
    bc90:	0f 85 ef 01 00 00    	jne    be85 <pythrust_main+0x3b5>
    bc96:	48 89 d8             	mov    %rbx,%rax
    bc99:	48 f7 2d 10 02 00 00 	imulq  0x210(%rip)        # beb0 <pythrust_main+0x3e0>
    bca0:	48 8d 34 1a          	lea    (%rdx,%rbx,1),%rsi
    bca4:	48 c1 fe 05          	sar    $0x5,%rsi
    bca8:	48 89 f7             	mov    %rsi,%rdi
    bcab:	48 c1 ef 3f          	shr    $0x3f,%rdi
    bcaf:	48 03 f7             	add    %rdi,%rsi
    bcb2:	48 6b fe 32          	imul   $0x32,%rsi,%rdi
    bcb6:	48 89 de             	mov    %rbx,%rsi
    bcb9:	48 2b f7             	sub    %rdi,%rsi
    bcbc:	48 3b df             	cmp    %rdi,%rbx
    bcbf:	40 0f 95 c7          	setne  %dil
    bcc3:	49 89 f1             	mov    %rsi,%r9
    bcc6:	49 83 f1 32          	xor    $0x32,%r9
    bcca:	4d 33 c0             	xor    %r8,%r8
    bccd:	49 c1 e9 3f          	shr    $0x3f,%r9
    bcd1:	41 84 f9             	test   %dil,%r9b
    bcd4:	4c 0f 45 05 dc 01 00 	cmovne 0x1dc(%rip),%r8        # beb8 <pythrust_main+0x3e8>
    bcdb:	00 
    bcdc:	4c 03 c6             	add    %rsi,%r8
    bcdf:	4d 85 c0             	test   %r8,%r8
    bce2:	0f 85 55 00 00 00    	jne    bd3d <pythrust_main+0x26d>
    bce8:	48 33 f6             	xor    %rsi,%rsi
    bceb:	48 8d 05 1e 44 00 00 	lea    0x441e(%rip),%rax        # 10110 <pt_str_index>
    bcf2:	4c 89 f7             	mov    %r14,%rdi
    bcf5:	ff d0                	call   *%rax
    bcf7:	48 89 c2             	mov    %rax,%rdx
    bcfa:	48 89 44 24 28       	mov    %rax,0x28(%rsp)
    bcff:	48 85 d0             	test   %rdx,%rax
    bd02:	0f 84 12 01 00 00    	je     be1a <pythrust_main+0x34a>
    bd08:	48 8d 0d d1 3b 00 00 	lea    0x3bd1(%rip),%rcx        # f8e0 <pt_str_concat>
    bd0f:	48 8b 74 24 28       	mov    0x28(%rsp),%rsi
    bd14:	4c 89 ff             	mov    %r15,%rdi
    bd17:	ff d1                	call   *%rcx
    bd19:	48 89 44 24 20       	mov    %rax,0x20(%rsp)
    bd1e:	48 8d 05 fb 3c 00 00 	lea    0x3cfb(%rip),%rax        # fa20 <pt_str_dec>
    bd25:	4c 89 ff             	mov    %r15,%rdi
    bd28:	ff d0                	call   *%rax
    bd2a:	48 8d 05 ef 3c 00 00 	lea    0x3cef(%rip),%rax        # fa20 <pt_str_dec>
    bd31:	48 8b 7c 24 28       	mov    0x28(%rsp),%rdi
    bd36:	ff d0                	call   *%rax
    bd38:	4c 8b 7c 24 20       	mov    0x20(%rsp),%r15
    bd3d:	48 89 d8             	mov    %rbx,%rax
    bd40:	48 83 c0 01          	add    $0x1,%rax
    bd44:	0f 90 c0             	seto   %al
    bd47:	84 c0                	test   %al,%al
    bd49:	0f 84 0a 00 00 00    	je     bd59 <pythrust_main+0x289>
    bd4f:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    bd54:	e9 19 00 00 00       	jmp    bd72 <pythrust_main+0x2a2>
    bd59:	48 83 c3 01          	add    $0x1,%rbx
    bd5d:	0f 90 c0             	seto   %al
    bd60:	84 c0                	test   %al,%al
    bd62:	0f 85 9b 00 00 00    	jne    be03 <pythrust_main+0x333>
    bd68:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    bd6d:	e9 b3 fd ff ff       	jmp    bb25 <pythrust_main+0x55>
    bd72:	48 8d 05 07 46 00 00 	lea    0x4607(%rip),%rax        # 10380 <pt_str_len>
    bd79:	4c 89 ff             	mov    %r15,%rdi
    bd7c:	ff d0                	call   *%rax
    bd7e:	48 89 c3             	mov    %rax,%rbx
    bd81:	48 8d 05 68 62 00 00 	lea    0x6268(%rip),%rax        # 11ff0 <pt_write_int>
    bd88:	48 8b 7c 24 10       	mov    0x10(%rsp),%rdi
    bd8d:	ff d0                	call   *%rax
    bd8f:	b8 20 00 00 00       	mov    $0x20,%eax
    bd94:	48 0f b6 f8          	movzbq %al,%rdi
    bd98:	48 8d 05 21 62 00 00 	lea    0x6221(%rip),%rax        # 11fc0 <pt_write_byte>
    bd9f:	ff d0                	call   *%rax
    bda1:	48 8d 05 48 62 00 00 	lea    0x6248(%rip),%rax        # 11ff0 <pt_write_int>
    bda8:	48 89 df             	mov    %rbx,%rdi
    bdab:	ff d0                	call   *%rax
    bdad:	b8 0a 00 00 00       	mov    $0xa,%eax
    bdb2:	48 0f b6 f8          	movzbq %al,%rdi
    bdb6:	48 8d 05 03 62 00 00 	lea    0x6203(%rip),%rax        # 11fc0 <pt_write_byte>
    bdbd:	ff d0                	call   *%rax
    bdbf:	48 8d 05 5a 3c 00 00 	lea    0x3c5a(%rip),%rax        # fa20 <pt_str_dec>
    bdc6:	4c 89 ff             	mov    %r15,%rdi
    bdc9:	ff d0                	call   *%rax
    bdcb:	48 8d 05 4e 3c 00 00 	lea    0x3c4e(%rip),%rax        # fa20 <pt_str_dec>
    bdd2:	48 8b 7c 24 18       	mov    0x18(%rsp),%rdi
    bdd7:	ff d0                	call   *%rax
    bdd9:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
    bdde:	4c 8b 64 24 58       	mov    0x58(%rsp),%r12
    bde3:	4c 8b 6c 24 60       	mov    0x60(%rsp),%r13
    bde8:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
    bded:	4c 8b 7c 24 70       	mov    0x70(%rsp),%r15
    bdf2:	48 81 c4 80 00 00 00 	add    $0x80,%rsp
    bdf9:	48 89 ec             	mov    %rbp,%rsp
    bdfc:	5d                   	pop    %rbp
    bdfd:	c3                   	ret
    bdfe:	e9 53 00 00 00       	jmp    be56 <pythrust_main+0x386>
    be03:	48 8d 3d d3 13 04 00 	lea    0x413d3(%rip),%rdi        # 4d1dd <.Ldata9>
    be0a:	be a3 00 00 00       	mov    $0xa3,%esi
    be0f:	48 8d 05 2a 61 00 00 	lea    0x612a(%rip),%rax        # 11f40 <pt_fail>
    be16:	ff d0                	call   *%rax
    be18:	0f 0b                	ud2
    be1a:	b8 01 00 00 00       	mov    $0x1,%eax
    be1f:	48 8d 35 fc 14 04 00 	lea    0x414fc(%rip),%rsi        # 4d322 <.Ldatab>
    be26:	ba 19 00 00 00       	mov    $0x19,%edx
    be2b:	48 8d 0d 09 15 04 00 	lea    0x41509(%rip),%rcx        # 4d33b <.Ldatac>
    be32:	41 b8 4e 00 00 00    	mov    $0x4e,%r8d
    be38:	4c 8d 0d 4a 15 04 00 	lea    0x4154a(%rip),%r9        # 4d389 <.Ldatad>
    be3f:	41 ba 33 00 00 00    	mov    $0x33,%r10d
    be45:	48 0f b6 f8          	movzbq %al,%rdi
    be49:	4c 89 14 24          	mov    %r10,(%rsp)
    be4d:	48 8d 05 ac 68 00 00 	lea    0x68ac(%rip),%rax        # 12700 <pt_raise_msg>
    be54:	ff d0                	call   *%rax
    be56:	48 8d 05 c3 3b 00 00 	lea    0x3bc3(%rip),%rax        # fa20 <pt_str_dec>
    be5d:	4c 89 ff             	mov    %r15,%rdi
    be60:	ff d0                	call   *%rax
    be62:	48 8d 05 b7 3b 00 00 	lea    0x3bb7(%rip),%rax        # fa20 <pt_str_dec>
    be69:	4c 89 f7             	mov    %r14,%rdi
    be6c:	ff d0                	call   *%rax
    be6e:	48 33 ff             	xor    %rdi,%rdi
    be71:	48 8d 05 a8 3b 00 00 	lea    0x3ba8(%rip),%rax        # fa20 <pt_str_dec>
    be78:	ff d0                	call   *%rax
    be7a:	48 8d 05 6f 69 00 00 	lea    0x696f(%rip),%rax        # 127f0 <pt_uncaught>
    be81:	ff d0                	call   *%rax
    be83:	0f 0b                	ud2
    be85:	48 8d 3d 40 12 04 00 	lea    0x41240(%rip),%rdi        # 4d0cc <.Ldata6>
    be8c:	be 9d 00 00 00       	mov    $0x9d,%esi
    be91:	48 8d 05 a8 60 00 00 	lea    0x60a8(%rip),%rax        # 11f40 <pt_fail>
    be98:	ff d0                	call   *%rax
    be9a:	0f 0b                	ud2
    be9c:	00 00                	add    %al,(%rax)
    be9e:	00 00                	add    %al,(%rax)
    bea0:	e9 a8 c0 17 57       	jmp    57187f4d <_end+0x57125425>
    bea5:	3f                   	(bad)
    bea6:	e8 a8 61 00 00       	call   12053 <pt_write_int+0x63>
    beab:	00 00                	add    %al,(%rax)
    bead:	00 00                	add    %al,(%rax)
    beaf:	00 0b                	add    %cl,(%rbx)
    beb1:	d7                   	xlat   %ds:(%rbx)
    beb2:	a3 70 3d 0a d7 a3 32 	movabs %eax,0x32a3d70a3d70
    beb9:	00 00 
    bebb:	00 00                	add    %al,(%rax)
    bebd:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:
