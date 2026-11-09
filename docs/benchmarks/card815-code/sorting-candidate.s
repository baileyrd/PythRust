
/workspace/card815-revised/candidate/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000b8e0 <pythrust_main>:
    b8e0:	55                   	push   %rbp
    b8e1:	48 89 e5             	mov    %rsp,%rbp
    b8e4:	48 81 ec 80 00 00 00 	sub    $0x80,%rsp
    b8eb:	48 89 5c 24 50       	mov    %rbx,0x50(%rsp)
    b8f0:	4c 89 64 24 58       	mov    %r12,0x58(%rsp)
    b8f5:	4c 89 6c 24 60       	mov    %r13,0x60(%rsp)
    b8fa:	4c 89 74 24 68       	mov    %r14,0x68(%rsp)
    b8ff:	4c 89 7c 24 70       	mov    %r15,0x70(%rsp)
    b904:	49 89 fd             	mov    %rdi,%r13
    b907:	48 8d 3d f2 36 04 00 	lea    0x436f2(%rip),%rdi        # 4f000 <.Ldata0>
    b90e:	be 02 00 00 00       	mov    $0x2,%esi
    b913:	48 33 db             	xor    %rbx,%rbx
    b916:	4c 8d 05 53 43 00 00 	lea    0x4353(%rip),%r8        # fc70 <pt_seq_new>
    b91d:	48 89 da             	mov    %rbx,%rdx
    b920:	41 ff d0             	call   *%r8
    b923:	48 89 44 24 48       	mov    %rax,0x48(%rsp)
    b928:	b8 39 30 00 00       	mov    $0x3039,%eax
    b92d:	48 81 fb e0 93 04 00 	cmp    $0x493e0,%rbx
    b934:	0f 8d e2 00 00 00    	jge    ba1c <pythrust_main+0x13c>
    b93a:	48 f7 2d 9f 05 00 00 	imulq  0x59f(%rip)        # bee0 <pythrust_main+0x600>
    b941:	0f 80 7e 05 00 00    	jo     bec5 <pythrust_main+0x5e5>
    b947:	48 05 39 30 00 00    	add    $0x3039,%rax
    b94d:	41 0f 90 c0          	seto   %r8b
    b951:	45 84 c0             	test   %r8b,%r8b
    b954:	0f 85 54 05 00 00    	jne    beae <pythrust_main+0x5ce>
    b95a:	49 89 c2             	mov    %rax,%r10
    b95d:	49 c1 fa 1e          	sar    $0x1e,%r10
    b961:	49 c1 ea 21          	shr    $0x21,%r10
    b965:	4c 03 d0             	add    %rax,%r10
    b968:	49 81 e2 00 00 00 80 	and    $0xffffffff80000000,%r10
    b96f:	49 89 c3             	mov    %rax,%r11
    b972:	4d 2b da             	sub    %r10,%r11
    b975:	49 3b c2             	cmp    %r10,%rax
    b978:	0f 95 c0             	setne  %al
    b97b:	4d 89 d9             	mov    %r11,%r9
    b97e:	4c 33 0d 63 05 00 00 	xor    0x563(%rip),%r9        # bee8 <pythrust_main+0x608>
    b985:	48 33 c9             	xor    %rcx,%rcx
    b988:	49 c1 e9 3f          	shr    $0x3f,%r9
    b98c:	41 84 c1             	test   %al,%r9b
    b98f:	48 89 c8             	mov    %rcx,%rax
    b992:	48 0f 45 05 4e 05 00 	cmovne 0x54e(%rip),%rax        # bee8 <pythrust_main+0x608>
    b999:	00 
    b99a:	49 03 c3             	add    %r11,%rax
    b99d:	49 89 c4             	mov    %rax,%r12
    b9a0:	48 f7 2d 49 05 00 00 	imulq  0x549(%rip)        # bef0 <pythrust_main+0x610>
    b9a7:	48 c1 fa 12          	sar    $0x12,%rdx
    b9ab:	48 89 d0             	mov    %rdx,%rax
    b9ae:	48 c1 e8 3f          	shr    $0x3f,%rax
    b9b2:	48 03 c2             	add    %rdx,%rax
    b9b5:	48 69 d0 40 42 0f 00 	imul   $0xf4240,%rax,%rdx
    b9bc:	4c 89 e0             	mov    %r12,%rax
    b9bf:	48 2b c2             	sub    %rdx,%rax
    b9c2:	4c 3b e2             	cmp    %rdx,%r12
    b9c5:	0f 95 c2             	setne  %dl
    b9c8:	48 89 c6             	mov    %rax,%rsi
    b9cb:	48 81 f6 40 42 0f 00 	xor    $0xf4240,%rsi
    b9d2:	48 c1 ee 3f          	shr    $0x3f,%rsi
    b9d6:	40 84 d6             	test   %dl,%sil
    b9d9:	48 0f 45 0d 17 05 00 	cmovne 0x517(%rip),%rcx        # bef8 <pythrust_main+0x618>
    b9e0:	00 
    b9e1:	48 8d 34 08          	lea    (%rax,%rcx,1),%rsi
    b9e5:	48 8d 05 54 43 00 00 	lea    0x4354(%rip),%rax        # fd40 <pt_seq_push>
    b9ec:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    b9f1:	ff d0                	call   *%rax
    b9f3:	48 89 d8             	mov    %rbx,%rax
    b9f6:	48 83 c0 01          	add    $0x1,%rax
    b9fa:	0f 90 c0             	seto   %al
    b9fd:	84 c0                	test   %al,%al
    b9ff:	0f 85 17 00 00 00    	jne    ba1c <pythrust_main+0x13c>
    ba05:	48 83 c3 01          	add    $0x1,%rbx
    ba09:	0f 90 c0             	seto   %al
    ba0c:	84 c0                	test   %al,%al
    ba0e:	0f 85 9b 02 00 00    	jne    bcaf <pythrust_main+0x3cf>
    ba14:	4c 89 e0             	mov    %r12,%rax
    ba17:	e9 11 ff ff ff       	jmp    b92d <pythrust_main+0x4d>
    ba1c:	48 8d 05 cd 41 00 00 	lea    0x41cd(%rip),%rax        # fbf0 <pt_seq_len>
    ba23:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    ba28:	ff d0                	call   *%rax
    ba2a:	48 89 c1             	mov    %rax,%rcx
    ba2d:	48 83 e9 01          	sub    $0x1,%rcx
    ba31:	0f 90 c0             	seto   %al
    ba34:	84 c0                	test   %al,%al
    ba36:	0f 85 5b 04 00 00    	jne    be97 <pythrust_main+0x5b7>
    ba3c:	4d 89 ef             	mov    %r13,%r15
    ba3f:	49 8b 5f 08          	mov    0x8(%r15),%rbx
    ba43:	48 81 fb e8 03 00 00 	cmp    $0x3e8,%rbx
    ba4a:	0f 8d 30 04 00 00    	jge    be80 <pythrust_main+0x5a0>
    ba50:	48 8d 43 01          	lea    0x1(%rbx),%rax
    ba54:	49 89 47 08          	mov    %rax,0x8(%r15)
    ba58:	48 33 d2             	xor    %rdx,%rdx
    ba5b:	48 8b 74 24 48       	mov    0x48(%rsp),%rsi
    ba60:	4c 89 ff             	mov    %r15,%rdi
    ba63:	e8 98 04 00 00       	call   bf00 <py1_quicksort>
    ba68:	49 89 5f 08          	mov    %rbx,0x8(%r15)
    ba6c:	49 0f b6 07          	movzbq (%r15),%rax
    ba70:	84 c0                	test   %al,%al
    ba72:	0f 85 ef 03 00 00    	jne    be67 <pythrust_main+0x587>
    ba78:	48 8d 05 71 41 00 00 	lea    0x4171(%rip),%rax        # fbf0 <pt_seq_len>
    ba7f:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    ba84:	ff d0                	call   *%rax
    ba86:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
    ba8b:	41 bc 01 00 00 00    	mov    $0x1,%r12d
    ba91:	bb 01 00 00 00       	mov    $0x1,%ebx
    ba96:	4c 8d 6c 24 10       	lea    0x10(%rsp),%r13
    ba9b:	4c 8d 74 24 18       	lea    0x18(%rsp),%r14
    baa0:	48 8b 4c 24 40       	mov    0x40(%rsp),%rcx
    baa5:	49 89 df             	mov    %rbx,%r15
    baa8:	4c 3b e1             	cmp    %rcx,%r12
    baab:	48 89 4c 24 40       	mov    %rcx,0x40(%rsp)
    bab0:	0f 8d 9b 00 00 00    	jge    bb51 <pythrust_main+0x271>
    bab6:	4c 89 e6             	mov    %r12,%rsi
    bab9:	48 83 ee 01          	sub    $0x1,%rsi
    babd:	0f 90 c0             	seto   %al
    bac0:	84 c0                	test   %al,%al
    bac2:	0f 85 88 03 00 00    	jne    be50 <pythrust_main+0x570>
    bac8:	48 8d 05 61 3f 00 00 	lea    0x3f61(%rip),%rax        # fa30 <pt_int_list_get>
    bacf:	4c 89 ea             	mov    %r13,%rdx
    bad2:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bad7:	ff d0                	call   *%rax
    bad9:	48 89 44 24 38       	mov    %rax,0x38(%rsp)
    bade:	48 0f b6 44 24 10    	movzbq 0x10(%rsp),%rax
    bae4:	3c 01                	cmp    $0x1,%al
    bae6:	0f 84 23 03 00 00    	je     be0f <pythrust_main+0x52f>
    baec:	48 8d 05 3d 3f 00 00 	lea    0x3f3d(%rip),%rax        # fa30 <pt_int_list_get>
    baf3:	4c 89 f2             	mov    %r14,%rdx
    baf6:	4c 89 e6             	mov    %r12,%rsi
    baf9:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bafe:	ff d0                	call   *%rax
    bb00:	48 0f b6 4c 24 18    	movzbq 0x18(%rsp),%rcx
    bb06:	40 80 f9 01          	rex cmp $0x1,%cl
    bb0a:	0f 84 be 02 00 00    	je     bdce <pythrust_main+0x4ee>
    bb10:	4c 8b 5c 24 38       	mov    0x38(%rsp),%r11
    bb15:	4c 3b d8             	cmp    %rax,%r11
    bb18:	0f 8e 08 00 00 00    	jle    bb26 <pythrust_main+0x246>
    bb1e:	bb 01 00 00 00       	mov    $0x1,%ebx
    bb23:	45 33 ff             	xor    %r15d,%r15d
    bb26:	4c 89 e0             	mov    %r12,%rax
    bb29:	48 83 c0 01          	add    $0x1,%rax
    bb2d:	0f 90 c0             	seto   %al
    bb30:	84 c0                	test   %al,%al
    bb32:	0f 85 19 00 00 00    	jne    bb51 <pythrust_main+0x271>
    bb38:	49 83 c4 01          	add    $0x1,%r12
    bb3c:	0f 90 c0             	seto   %al
    bb3f:	84 c0                	test   %al,%al
    bb41:	0f 85 7f 01 00 00    	jne    bcc6 <pythrust_main+0x3e6>
    bb47:	48 8b 4c 24 40       	mov    0x40(%rsp),%rcx
    bb4c:	e9 57 ff ff ff       	jmp    baa8 <pythrust_main+0x1c8>
    bb51:	84 db                	test   %bl,%bl
    bb53:	0f 84 5e 02 00 00    	je     bdb7 <pythrust_main+0x4d7>
    bb59:	48 33 f6             	xor    %rsi,%rsi
    bb5c:	48 8d 54 24 20       	lea    0x20(%rsp),%rdx
    bb61:	48 8d 0d c8 3e 00 00 	lea    0x3ec8(%rip),%rcx        # fa30 <pt_int_list_get>
    bb68:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bb6d:	ff d1                	call   *%rcx
    bb6f:	49 89 c6             	mov    %rax,%r14
    bb72:	48 0f b6 4c 24 20    	movzbq 0x20(%rsp),%rcx
    bb78:	40 80 f9 01          	rex cmp $0x1,%cl
    bb7c:	0f 84 f4 01 00 00    	je     bd76 <pythrust_main+0x496>
    bb82:	48 8d 35 67 40 00 00 	lea    0x4067(%rip),%rsi        # fbf0 <pt_seq_len>
    bb89:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bb8e:	ff d6                	call   *%rsi
    bb90:	48 89 c6             	mov    %rax,%rsi
    bb93:	48 d1 fe             	sar    $1,%rsi
    bb96:	48 8d 54 24 28       	lea    0x28(%rsp),%rdx
    bb9b:	4c 8d 05 8e 3e 00 00 	lea    0x3e8e(%rip),%r8        # fa30 <pt_int_list_get>
    bba2:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bba7:	41 ff d0             	call   *%r8
    bbaa:	48 89 c3             	mov    %rax,%rbx
    bbad:	48 0f b6 74 24 28    	movzbq 0x28(%rsp),%rsi
    bbb3:	40 80 fe 01          	cmp    $0x1,%sil
    bbb7:	0f 84 78 01 00 00    	je     bd35 <pythrust_main+0x455>
    bbbd:	4c 8d 05 2c 40 00 00 	lea    0x402c(%rip),%r8        # fbf0 <pt_seq_len>
    bbc4:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bbc9:	41 ff d0             	call   *%r8
    bbcc:	48 89 c6             	mov    %rax,%rsi
    bbcf:	48 83 ee 01          	sub    $0x1,%rsi
    bbd3:	41 0f 90 c0          	seto   %r8b
    bbd7:	45 84 c0             	test   %r8b,%r8b
    bbda:	0f 85 3e 01 00 00    	jne    bd1e <pythrust_main+0x43e>
    bbe0:	48 8d 54 24 30       	lea    0x30(%rsp),%rdx
    bbe5:	4c 8d 0d 44 3e 00 00 	lea    0x3e44(%rip),%r9        # fa30 <pt_int_list_get>
    bbec:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bbf1:	41 ff d1             	call   *%r9
    bbf4:	49 89 c4             	mov    %rax,%r12
    bbf7:	4c 0f b6 4c 24 30    	movzbq 0x30(%rsp),%r9
    bbfd:	41 80 f9 01          	cmp    $0x1,%r9b
    bc01:	0f 84 d6 00 00 00    	je     bcdd <pythrust_main+0x3fd>
    bc07:	49 0f b6 ff          	movzbq %r15b,%rdi
    bc0b:	48 8d 05 4e 6a 00 00 	lea    0x6a4e(%rip),%rax        # 12660 <pt_write_bool>
    bc12:	ff d0                	call   *%rax
    bc14:	41 bd 20 00 00 00    	mov    $0x20,%r13d
    bc1a:	49 0f b6 fd          	movzbq %r13b,%rdi
    bc1e:	48 8d 05 7b 6a 00 00 	lea    0x6a7b(%rip),%rax        # 126a0 <pt_write_byte>
    bc25:	ff d0                	call   *%rax
    bc27:	48 8d 05 a2 6a 00 00 	lea    0x6aa2(%rip),%rax        # 126d0 <pt_write_int>
    bc2e:	4c 89 f7             	mov    %r14,%rdi
    bc31:	ff d0                	call   *%rax
    bc33:	49 0f b6 fd          	movzbq %r13b,%rdi
    bc37:	48 8d 05 62 6a 00 00 	lea    0x6a62(%rip),%rax        # 126a0 <pt_write_byte>
    bc3e:	ff d0                	call   *%rax
    bc40:	48 8d 05 89 6a 00 00 	lea    0x6a89(%rip),%rax        # 126d0 <pt_write_int>
    bc47:	48 89 df             	mov    %rbx,%rdi
    bc4a:	ff d0                	call   *%rax
    bc4c:	49 0f b6 fd          	movzbq %r13b,%rdi
    bc50:	48 8d 05 49 6a 00 00 	lea    0x6a49(%rip),%rax        # 126a0 <pt_write_byte>
    bc57:	ff d0                	call   *%rax
    bc59:	48 8d 05 70 6a 00 00 	lea    0x6a70(%rip),%rax        # 126d0 <pt_write_int>
    bc60:	4c 89 e7             	mov    %r12,%rdi
    bc63:	ff d0                	call   *%rax
    bc65:	b8 0a 00 00 00       	mov    $0xa,%eax
    bc6a:	48 0f b6 f8          	movzbq %al,%rdi
    bc6e:	48 8d 05 2b 6a 00 00 	lea    0x6a2b(%rip),%rax        # 126a0 <pt_write_byte>
    bc75:	ff d0                	call   *%rax
    bc77:	48 8d 05 92 3e 00 00 	lea    0x3e92(%rip),%rax        # fb10 <pt_seq_dec>
    bc7e:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bc83:	ff d0                	call   *%rax
    bc85:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
    bc8a:	4c 8b 64 24 58       	mov    0x58(%rsp),%r12
    bc8f:	4c 8b 6c 24 60       	mov    0x60(%rsp),%r13
    bc94:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
    bc99:	4c 8b 7c 24 70       	mov    0x70(%rsp),%r15
    bc9e:	48 81 c4 80 00 00 00 	add    $0x80,%rsp
    bca5:	48 89 ec             	mov    %rbp,%rsp
    bca8:	5d                   	pop    %rbp
    bca9:	c3                   	ret
    bcaa:	e9 b8 01 00 00       	jmp    be67 <pythrust_main+0x587>
    bcaf:	48 8d 3d 20 36 04 00 	lea    0x43620(%rip),%rdi        # 4f2d6 <.Ldata8>
    bcb6:	be 97 00 00 00       	mov    $0x97,%esi
    bcbb:	48 8d 05 3e 69 00 00 	lea    0x693e(%rip),%rax        # 12600 <pt_fail>
    bcc2:	ff d0                	call   *%rax
    bcc4:	0f 0b                	ud2
    bcc6:	48 8d 3d 13 39 04 00 	lea    0x43913(%rip),%rdi        # 4f5e0 <.Ldataf>
    bccd:	be 9f 00 00 00       	mov    $0x9f,%esi
    bcd2:	48 8d 05 27 69 00 00 	lea    0x6927(%rip),%rax        # 12600 <pt_fail>
    bcd9:	ff d0                	call   *%rax
    bcdb:	0f 0b                	ud2
    bcdd:	b8 01 00 00 00       	mov    $0x1,%eax
    bce2:	48 8d 35 6d 38 04 00 	lea    0x4386d(%rip),%rsi        # 4f556 <.Ldatac>
    bce9:	ba 17 00 00 00       	mov    $0x17,%edx
    bcee:	48 8d 0d 45 3d 04 00 	lea    0x43d45(%rip),%rcx        # 4fa3a <.Ldata19>
    bcf5:	41 b8 44 00 00 00    	mov    $0x44,%r8d
    bcfb:	4c 8d 0d 7c 3d 04 00 	lea    0x43d7c(%rip),%r9        # 4fa7e <.Ldata1a>
    bd02:	41 ba 6c 00 00 00    	mov    $0x6c,%r10d
    bd08:	48 0f b6 f8          	movzbq %al,%rdi
    bd0c:	4c 89 14 24          	mov    %r10,(%rsp)
    bd10:	48 8d 05 19 72 00 00 	lea    0x7219(%rip),%rax        # 12f30 <pt_raise_msg>
    bd17:	ff d0                	call   *%rax
    bd19:	e9 49 01 00 00       	jmp    be67 <pythrust_main+0x587>
    bd1e:	48 8d 3d 47 3c 04 00 	lea    0x43c47(%rip),%rdi        # 4f96c <.Ldata18>
    bd25:	be ce 00 00 00       	mov    $0xce,%esi
    bd2a:	48 8d 05 cf 68 00 00 	lea    0x68cf(%rip),%rax        # 12600 <pt_fail>
    bd31:	ff d0                	call   *%rax
    bd33:	0f 0b                	ud2
    bd35:	b8 01 00 00 00       	mov    $0x1,%eax
    bd3a:	48 8d 35 15 38 04 00 	lea    0x43815(%rip),%rsi        # 4f556 <.Ldatac>
    bd41:	ba 17 00 00 00       	mov    $0x17,%edx
    bd46:	48 8d 0d 80 3b 04 00 	lea    0x43b80(%rip),%rcx        # 4f8cd <.Ldata16>
    bd4d:	41 b8 44 00 00 00    	mov    $0x44,%r8d
    bd53:	4c 8d 0d b7 3b 04 00 	lea    0x43bb7(%rip),%r9        # 4f911 <.Ldata17>
    bd5a:	41 ba 5b 00 00 00    	mov    $0x5b,%r10d
    bd60:	48 0f b6 f8          	movzbq %al,%rdi
    bd64:	4c 89 14 24          	mov    %r10,(%rsp)
    bd68:	48 8d 05 c1 71 00 00 	lea    0x71c1(%rip),%rax        # 12f30 <pt_raise_msg>
    bd6f:	ff d0                	call   *%rax
    bd71:	e9 f1 00 00 00       	jmp    be67 <pythrust_main+0x587>
    bd76:	b8 01 00 00 00       	mov    $0x1,%eax
    bd7b:	48 8d 35 d4 37 04 00 	lea    0x437d4(%rip),%rsi        # 4f556 <.Ldatac>
    bd82:	ba 17 00 00 00       	mov    $0x17,%edx
    bd87:	48 8d 0d b2 3a 04 00 	lea    0x43ab2(%rip),%rcx        # 4f840 <.Ldata14>
    bd8e:	41 b8 44 00 00 00    	mov    $0x44,%r8d
    bd94:	4c 8d 0d e9 3a 04 00 	lea    0x43ae9(%rip),%r9        # 4f884 <.Ldata15>
    bd9b:	41 ba 49 00 00 00    	mov    $0x49,%r10d
    bda1:	48 0f b6 f8          	movzbq %al,%rdi
    bda5:	4c 89 14 24          	mov    %r10,(%rsp)
    bda9:	48 8d 05 80 71 00 00 	lea    0x7180(%rip),%rax        # 12f30 <pt_raise_msg>
    bdb0:	ff d0                	call   *%rax
    bdb2:	e9 b0 00 00 00       	jmp    be67 <pythrust_main+0x587>
    bdb7:	48 8d 3d da 39 04 00 	lea    0x439da(%rip),%rdi        # 4f798 <.Ldata13>
    bdbe:	be a8 00 00 00       	mov    $0xa8,%esi
    bdc3:	48 8d 05 36 68 00 00 	lea    0x6836(%rip),%rax        # 12600 <pt_fail>
    bdca:	ff d0                	call   *%rax
    bdcc:	0f 0b                	ud2
    bdce:	b8 01 00 00 00       	mov    $0x1,%eax
    bdd3:	48 8d 35 7c 37 04 00 	lea    0x4377c(%rip),%rsi        # 4f556 <.Ldatac>
    bdda:	ba 17 00 00 00       	mov    $0x17,%edx
    bddf:	48 8d 0d 36 39 04 00 	lea    0x43936(%rip),%rcx        # 4f71c <.Ldata11>
    bde6:	41 b8 44 00 00 00    	mov    $0x44,%r8d
    bdec:	4c 8d 0d 6d 39 04 00 	lea    0x4396d(%rip),%r9        # 4f760 <.Ldata12>
    bdf3:	41 ba 38 00 00 00    	mov    $0x38,%r10d
    bdf9:	48 0f b6 f8          	movzbq %al,%rdi
    bdfd:	4c 89 14 24          	mov    %r10,(%rsp)
    be01:	48 8d 05 28 71 00 00 	lea    0x7128(%rip),%rax        # 12f30 <pt_raise_msg>
    be08:	ff d0                	call   *%rax
    be0a:	e9 58 00 00 00       	jmp    be67 <pythrust_main+0x587>
    be0f:	b8 01 00 00 00       	mov    $0x1,%eax
    be14:	48 8d 35 3b 37 04 00 	lea    0x4373b(%rip),%rsi        # 4f556 <.Ldatac>
    be1b:	ba 17 00 00 00       	mov    $0x17,%edx
    be20:	48 8d 0d 46 37 04 00 	lea    0x43746(%rip),%rcx        # 4f56d <.Ldatad>
    be27:	41 b8 43 00 00 00    	mov    $0x43,%r8d
    be2d:	4c 8d 0d 7c 37 04 00 	lea    0x4377c(%rip),%r9        # 4f5b0 <.Ldatae>
    be34:	41 ba 30 00 00 00    	mov    $0x30,%r10d
    be3a:	48 0f b6 f8          	movzbq %al,%rdi
    be3e:	4c 89 14 24          	mov    %r10,(%rsp)
    be42:	48 8d 05 e7 70 00 00 	lea    0x70e7(%rip),%rax        # 12f30 <pt_raise_msg>
    be49:	ff d0                	call   *%rax
    be4b:	e9 17 00 00 00       	jmp    be67 <pythrust_main+0x587>
    be50:	48 8d 3d 6d 36 04 00 	lea    0x4366d(%rip),%rdi        # 4f4c4 <.Ldatab>
    be57:	be 92 00 00 00       	mov    $0x92,%esi
    be5c:	48 8d 05 9d 67 00 00 	lea    0x679d(%rip),%rax        # 12600 <pt_fail>
    be63:	ff d0                	call   *%rax
    be65:	0f 0b                	ud2
    be67:	48 8d 05 a2 3c 00 00 	lea    0x3ca2(%rip),%rax        # fb10 <pt_seq_dec>
    be6e:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    be73:	ff d0                	call   *%rax
    be75:	48 8d 05 a4 71 00 00 	lea    0x71a4(%rip),%rax        # 13020 <pt_uncaught>
    be7c:	ff d0                	call   *%rax
    be7e:	0f 0b                	ud2
    be80:	48 8d 3d 89 35 04 00 	lea    0x43589(%rip),%rdi        # 4f410 <.Ldataa>
    be87:	be b4 00 00 00       	mov    $0xb4,%esi
    be8c:	48 8d 05 6d 67 00 00 	lea    0x676d(%rip),%rax        # 12600 <pt_fail>
    be93:	ff d0                	call   *%rax
    be95:	0f 0b                	ud2
    be97:	48 8d 3d cf 34 04 00 	lea    0x434cf(%rip),%rdi        # 4f36d <.Ldata9>
    be9e:	be a3 00 00 00       	mov    $0xa3,%esi
    bea3:	48 8d 05 56 67 00 00 	lea    0x6756(%rip),%rax        # 12600 <pt_fail>
    beaa:	ff d0                	call   *%rax
    beac:	0f 0b                	ud2
    beae:	48 8d 3d 07 32 04 00 	lea    0x43207(%rip),%rdi        # 4f0bc <.Ldata2>
    beb5:	be c3 00 00 00       	mov    $0xc3,%esi
    beba:	48 8d 05 3f 67 00 00 	lea    0x673f(%rip),%rax        # 12600 <pt_fail>
    bec1:	ff d0                	call   *%rax
    bec3:	0f 0b                	ud2
    bec5:	48 8d 3d 36 31 04 00 	lea    0x43136(%rip),%rdi        # 4f002 <.Ldata1>
    becc:	be ba 00 00 00       	mov    $0xba,%esi
    bed1:	48 8d 05 28 67 00 00 	lea    0x6728(%rip),%rax        # 12600 <pt_fail>
    bed8:	ff d0                	call   *%rax
    beda:	0f 0b                	ud2
    bedc:	00 00                	add    %al,(%rax)
    bede:	00 00                	add    %al,(%rax)
    bee0:	6d                   	insl   (%dx),%es:(%rdi)
    bee1:	4e c6 41 00 00       	rex.WRX movb $0x0,0x0(%rcx)
    bee6:	00 00                	add    %al,(%rax)
    bee8:	00 00                	add    %al,(%rax)
    beea:	00 80 00 00 00 00    	add    %al,0x0(%rax)
    bef0:	db 34 b6             	(bad) (%rsi,%rsi,4)
    bef3:	d7                   	xlat   %ds:(%rbx)
    bef4:	82                   	(bad)
    bef5:	de 1b                	ficomps (%rbx)
    bef7:	43                   	rex.XB
    bef8:	40                   	rex
    bef9:	42 0f 00 00          	rex.X sldt (%rax)
    befd:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:

/workspace/card815-revised/candidate/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000fa30 <pt_int_list_get>:
    fa30:	53                   	push   %rbx
    fa31:	48 83 ec 20          	sub    $0x20,%rsp
    fa35:	48 89 d3             	mov    %rdx,%rbx
    fa38:	48 89 f2             	mov    %rsi,%rdx
    fa3b:	48 89 fe             	mov    %rdi,%rsi
    fa3e:	48 8b 47 18          	mov    0x18(%rdi),%rax
    fa42:	48 85 c0             	test   %rax,%rax
    fa45:	75 33                	jne    fa7a <pt_int_list_get+0x4a>
    fa47:	48 8b 4e 30          	mov    0x30(%rsi),%rcx
    fa4b:	48 89 d0             	mov    %rdx,%rax
    fa4e:	48 c1 f8 3f          	sar    $0x3f,%rax
    fa52:	48 21 c8             	and    %rcx,%rax
    fa55:	48 01 d0             	add    %rdx,%rax
    fa58:	0f 99 c2             	setns  %dl
    fa5b:	48 39 c8             	cmp    %rcx,%rax
    fa5e:	0f 92 c1             	setb   %cl
    fa61:	20 d1                	and    %dl,%cl
    fa63:	80 f9 01             	cmp    $0x1,%cl
    fa66:	75 2e                	jne    fa96 <pt_int_list_get+0x66>
    fa68:	48 8b 4e 28          	mov    0x28(%rsi),%rcx
    fa6c:	48 8b 04 c1          	mov    (%rcx,%rax,8),%rax
    fa70:	31 c9                	xor    %ecx,%ecx
    fa72:	88 0b                	mov    %cl,(%rbx)
    fa74:	48 83 c4 20          	add    $0x20,%rsp
    fa78:	5b                   	pop    %rbx
    fa79:	c3                   	ret
    fa7a:	48 83 c6 18          	add    $0x18,%rsi
    fa7e:	48 8d 7c 24 08       	lea    0x8(%rsp),%rdi
    fa83:	e8 d8 65 ff ff       	call   6060 <_RINvMs1_NtCsk1kE6r7WBHz_11pythrust_rt4syncINtB6_4SlotINtNtCs40k4W9msRzi_5alloc3vec3VecyEE12get_borrowedNCNvNtB8_3seq15pt_int_list_get0EB8_>
    fa88:	83 7c 24 08 01       	cmpl   $0x1,0x8(%rsp)
    fa8d:	75 07                	jne    fa96 <pt_int_list_get+0x66>
    fa8f:	48 8b 44 24 18       	mov    0x18(%rsp),%rax
    fa94:	eb da                	jmp    fa70 <pt_int_list_get+0x40>
    fa96:	b1 01                	mov    $0x1,%cl
    fa98:	31 c0                	xor    %eax,%eax
    fa9a:	88 0b                	mov    %cl,(%rbx)
    fa9c:	48 83 c4 20          	add    $0x20,%rsp
    faa0:	5b                   	pop    %rbx
    faa1:	c3                   	ret
    faa2:	ff 15 e0 5d 05 00    	call   *0x55de0(%rip)        # 65888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card815-revised/candidate/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000fab0 <pt_int_list_set>:
    fab0:	50                   	push   %rax
    fab1:	48 8b 47 18          	mov    0x18(%rdi),%rax
    fab5:	48 85 c0             	test   %rax,%rax
    fab8:	75 33                	jne    faed <pt_int_list_set+0x3d>
    faba:	48 8b 47 30          	mov    0x30(%rdi),%rax
    fabe:	48 89 f1             	mov    %rsi,%rcx
    fac1:	48 c1 f9 3f          	sar    $0x3f,%rcx
    fac5:	48 21 c1             	and    %rax,%rcx
    fac8:	48 01 f1             	add    %rsi,%rcx
    facb:	40 0f 99 c6          	setns  %sil
    facf:	48 39 c1             	cmp    %rax,%rcx
    fad2:	41 0f 92 c0          	setb   %r8b
    fad6:	41 20 f0             	and    %sil,%r8b
    fad9:	b0 01                	mov    $0x1,%al
    fadb:	41 80 f8 01          	cmp    $0x1,%r8b
    fadf:	75 0a                	jne    faeb <pt_int_list_set+0x3b>
    fae1:	48 8b 47 28          	mov    0x28(%rdi),%rax
    fae5:	48 89 14 c8          	mov    %rdx,(%rax,%rcx,8)
    fae9:	31 c0                	xor    %eax,%eax
    faeb:	59                   	pop    %rcx
    faec:	c3                   	ret
    faed:	48 83 c7 18          	add    $0x18,%rdi
    faf1:	e8 2a 67 ff ff       	call   6220 <_RINvMs1_NtCsk1kE6r7WBHz_11pythrust_rt4syncINtB6_4SlotINtNtCs40k4W9msRzi_5alloc3vec3VecyEE16replace_borrowedNCNvNtB8_3seq15pt_int_list_set0EB8_>
    faf6:	48 83 f8 01          	cmp    $0x1,%rax
    fafa:	0f 95 c0             	setne  %al
    fafd:	59                   	pop    %rcx
    fafe:	c3                   	ret
    faff:	ff 15 83 5d 05 00    	call   *0x55d83(%rip)        # 65888 <_DYNAMIC+0x328>

Disassembly of section .fini:

/workspace/card815-revised/candidate/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000bf00 <py1_quicksort>:
    bf00:	55                   	push   %rbp
    bf01:	48 89 e5             	mov    %rsp,%rbp
    bf04:	48 81 ec b0 00 00 00 	sub    $0xb0,%rsp
    bf0b:	48 89 9c 24 80 00 00 	mov    %rbx,0x80(%rsp)
    bf12:	00 
    bf13:	4c 89 a4 24 88 00 00 	mov    %r12,0x88(%rsp)
    bf1a:	00 
    bf1b:	4c 89 ac 24 90 00 00 	mov    %r13,0x90(%rsp)
    bf22:	00 
    bf23:	4c 89 b4 24 98 00 00 	mov    %r14,0x98(%rsp)
    bf2a:	00 
    bf2b:	4c 89 bc 24 a0 00 00 	mov    %r15,0xa0(%rsp)
    bf32:	00 
    bf33:	49 89 ff             	mov    %rdi,%r15
    bf36:	48 89 74 24 38       	mov    %rsi,0x38(%rsp)
    bf3b:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    bf40:	48 89 4c 24 58       	mov    %rcx,0x58(%rsp)
    bf45:	48 8d 4c 24 10       	lea    0x10(%rsp),%rcx
    bf4a:	48 89 4c 24 78       	mov    %rcx,0x78(%rsp)
    bf4f:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    bf54:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    bf59:	48 3b d1             	cmp    %rcx,%rdx
    bf5c:	0f 8d c0 03 00 00    	jge    c322 <py1_quicksort+0x422>
    bf62:	48 89 d6             	mov    %rdx,%rsi
    bf65:	48 03 f1             	add    %rcx,%rsi
    bf68:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    bf6d:	48 89 4c 24 58       	mov    %rcx,0x58(%rsp)
    bf72:	0f 90 c0             	seto   %al
    bf75:	84 c0                	test   %al,%al
    bf77:	0f 85 2b 08 00 00    	jne    c7a8 <py1_quicksort+0x8a8>
    bf7d:	48 d1 fe             	sar    $1,%rsi
    bf80:	48 8d 05 a9 3a 00 00 	lea    0x3aa9(%rip),%rax        # fa30 <pt_int_list_get>
    bf87:	48 8b 54 24 78       	mov    0x78(%rsp),%rdx
    bf8c:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    bf91:	ff d0                	call   *%rax
    bf93:	48 89 44 24 70       	mov    %rax,0x70(%rsp)
    bf98:	48 0f b6 44 24 10    	movzbq 0x10(%rsp),%rax
    bf9e:	3c 01                	cmp    $0x1,%al
    bfa0:	0f 84 c1 07 00 00    	je     c767 <py1_quicksort+0x867>
    bfa6:	41 be 01 00 00 00    	mov    $0x1,%r14d
    bfac:	48 8d 5c 24 28       	lea    0x28(%rsp),%rbx
    bfb1:	48 89 5c 24 68       	mov    %rbx,0x68(%rsp)
    bfb6:	4c 8d 64 24 30       	lea    0x30(%rsp),%r12
    bfbb:	4c 89 64 24 60       	mov    %r12,0x60(%rsp)
    bfc0:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    bfc5:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    bfca:	4d 89 f5             	mov    %r14,%r13
    bfcd:	45 84 f6             	test   %r14b,%r14b
    bfd0:	0f 84 7a 07 00 00    	je     c750 <py1_quicksort+0x850>
    bfd6:	45 84 ed             	test   %r13b,%r13b
    bfd9:	0f 84 5a 07 00 00    	je     c739 <py1_quicksort+0x839>
    bfdf:	48 8b 44 24 40       	mov    0x40(%rsp),%rax
    bfe4:	48 3b c1             	cmp    %rcx,%rax
    bfe7:	0f 8e 21 01 00 00    	jle    c10e <py1_quicksort+0x20e>
    bfed:	45 84 ed             	test   %r13b,%r13b
    bff0:	0f 84 2f 04 00 00    	je     c425 <py1_quicksort+0x525>
    bff6:	48 89 c8             	mov    %rcx,%rax
    bff9:	48 2b c2             	sub    %rdx,%rax
    bffc:	48 89 4c 24 48       	mov    %rcx,0x48(%rsp)
    c001:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
    c006:	0f 90 c1             	seto   %cl
    c009:	84 c9                	test   %cl,%cl
    c00b:	0f 85 fd 03 00 00    	jne    c40e <py1_quicksort+0x50e>
    c011:	45 84 f6             	test   %r14b,%r14b
    c014:	0f 84 dd 03 00 00    	je     c3f7 <py1_quicksort+0x4f7>
    c01a:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c01f:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c024:	48 2b ca             	sub    %rdx,%rcx
    c027:	0f 90 c2             	seto   %dl
    c02a:	84 d2                	test   %dl,%dl
    c02c:	0f 85 ae 03 00 00    	jne    c3e0 <py1_quicksort+0x4e0>
    c032:	48 3b c1             	cmp    %rcx,%rax
    c035:	0f 8c 72 00 00 00    	jl     c0ad <py1_quicksort+0x1ad>
    c03b:	45 84 f6             	test   %r14b,%r14b
    c03e:	0f 84 40 03 00 00    	je     c384 <py1_quicksort+0x484>
    c044:	4d 89 fc             	mov    %r15,%r12
    c047:	4d 8b 74 24 08       	mov    0x8(%r12),%r14
    c04c:	49 81 fe e8 03 00 00 	cmp    $0x3e8,%r14
    c053:	0f 8d 14 03 00 00    	jge    c36d <py1_quicksort+0x46d>
    c059:	4d 8d 4e 01          	lea    0x1(%r14),%r9
    c05d:	4d 89 4c 24 08       	mov    %r9,0x8(%r12)
    c062:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c067:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c06c:	48 8b 74 24 38       	mov    0x38(%rsp),%rsi
    c071:	4c 89 e7             	mov    %r12,%rdi
    c074:	e8 87 fe ff ff       	call   bf00 <py1_quicksort>
    c079:	4d 89 74 24 08       	mov    %r14,0x8(%r12)
    c07e:	49 0f b6 04 24       	movzbq (%r12),%rax
    c083:	84 c0                	test   %al,%al
    c085:	0f 85 97 02 00 00    	jne    c322 <py1_quicksort+0x422>
    c08b:	45 84 ed             	test   %r13b,%r13b
    c08e:	0f 84 c2 02 00 00    	je     c356 <py1_quicksort+0x456>
    c094:	48 8b 44 24 50       	mov    0x50(%rsp),%rax
    c099:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
    c09e:	48 8b 44 24 48       	mov    0x48(%rsp),%rax
    c0a3:	48 89 44 24 58       	mov    %rax,0x58(%rsp)
    c0a8:	e9 59 00 00 00       	jmp    c106 <py1_quicksort+0x206>
    c0ad:	4d 89 fc             	mov    %r15,%r12
    c0b0:	45 84 ed             	test   %r13b,%r13b
    c0b3:	0f 84 10 03 00 00    	je     c3c9 <py1_quicksort+0x4c9>
    c0b9:	49 8b 5c 24 08       	mov    0x8(%r12),%rbx
    c0be:	48 81 fb e8 03 00 00 	cmp    $0x3e8,%rbx
    c0c5:	0f 8d e7 02 00 00    	jge    c3b2 <py1_quicksort+0x4b2>
    c0cb:	48 8d 43 01          	lea    0x1(%rbx),%rax
    c0cf:	49 89 44 24 08       	mov    %rax,0x8(%r12)
    c0d4:	48 8b 4c 24 48       	mov    0x48(%rsp),%rcx
    c0d9:	48 8b 54 24 50       	mov    0x50(%rsp),%rdx
    c0de:	48 8b 74 24 38       	mov    0x38(%rsp),%rsi
    c0e3:	4c 89 e7             	mov    %r12,%rdi
    c0e6:	e8 15 fe ff ff       	call   bf00 <py1_quicksort>
    c0eb:	49 89 5c 24 08       	mov    %rbx,0x8(%r12)
    c0f0:	49 0f b6 04 24       	movzbq (%r12),%rax
    c0f5:	84 c0                	test   %al,%al
    c0f7:	0f 85 25 02 00 00    	jne    c322 <py1_quicksort+0x422>
    c0fd:	45 84 f6             	test   %r14b,%r14b
    c100:	0f 84 95 02 00 00    	je     c39b <py1_quicksort+0x49b>
    c106:	4d 89 e7             	mov    %r12,%r15
    c109:	e9 41 fe ff ff       	jmp    bf4f <py1_quicksort+0x4f>
    c10e:	4d 89 fc             	mov    %r15,%r12
    c111:	48 89 4c 24 48       	mov    %rcx,0x48(%rsp)
    c116:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
    c11b:	48 8d 5c 24 18       	lea    0x18(%rsp),%rbx
    c120:	45 84 f6             	test   %r14b,%r14b
    c123:	0f 84 f9 05 00 00    	je     c722 <py1_quicksort+0x822>
    c129:	48 8d 05 00 39 00 00 	lea    0x3900(%rip),%rax        # fa30 <pt_int_list_get>
    c130:	48 89 da             	mov    %rbx,%rdx
    c133:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c138:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c13d:	ff d0                	call   *%rax
    c13f:	48 0f b6 4c 24 18    	movzbq 0x18(%rsp),%rcx
    c145:	40 80 f9 01          	rex cmp $0x1,%cl
    c149:	0f 84 92 05 00 00    	je     c6e1 <py1_quicksort+0x7e1>
    c14f:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
    c154:	48 3b c1             	cmp    %rcx,%rax
    c157:	0f 8c 8e 01 00 00    	jl     c2eb <py1_quicksort+0x3eb>
    c15d:	48 8d 5c 24 20       	lea    0x20(%rsp),%rbx
    c162:	45 84 ed             	test   %r13b,%r13b
    c165:	0f 84 31 05 00 00    	je     c69c <py1_quicksort+0x79c>
    c16b:	4c 8d 05 be 38 00 00 	lea    0x38be(%rip),%r8        # fa30 <pt_int_list_get>
    c172:	4c 8b 7c 24 48       	mov    0x48(%rsp),%r15
    c177:	48 89 da             	mov    %rbx,%rdx
    c17a:	4c 89 fe             	mov    %r15,%rsi
    c17d:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c182:	41 ff d0             	call   *%r8
    c185:	48 0f b6 74 24 20    	movzbq 0x20(%rsp),%rsi
    c18b:	40 80 fe 01          	cmp    $0x1,%sil
    c18f:	0f 84 c6 04 00 00    	je     c65b <py1_quicksort+0x75b>
    c195:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
    c19a:	48 3b c1             	cmp    %rcx,%rax
    c19d:	0f 8f 20 01 00 00    	jg     c2c3 <py1_quicksort+0x3c3>
    c1a3:	45 84 f6             	test   %r14b,%r14b
    c1a6:	0f 84 6a 04 00 00    	je     c616 <py1_quicksort+0x716>
    c1ac:	45 84 ed             	test   %r13b,%r13b
    c1af:	0f 84 4a 04 00 00    	je     c5ff <py1_quicksort+0x6ff>
    c1b5:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c1ba:	49 3b d7             	cmp    %r15,%rdx
    c1bd:	0f 8f f0 00 00 00    	jg     c2b3 <py1_quicksort+0x3b3>
    c1c3:	45 84 f6             	test   %r14b,%r14b
    c1c6:	0f 84 1c 04 00 00    	je     c5e8 <py1_quicksort+0x6e8>
    c1cc:	48 8d 05 5d 38 00 00 	lea    0x385d(%rip),%rax        # fa30 <pt_int_list_get>
    c1d3:	48 8b 54 24 68       	mov    0x68(%rsp),%rdx
    c1d8:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c1dd:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c1e2:	ff d0                	call   *%rax
    c1e4:	48 89 c3             	mov    %rax,%rbx
    c1e7:	48 0f b6 44 24 28    	movzbq 0x28(%rsp),%rax
    c1ed:	3c 01                	cmp    $0x1,%al
    c1ef:	0f 84 b2 03 00 00    	je     c5a7 <py1_quicksort+0x6a7>
    c1f5:	45 84 ed             	test   %r13b,%r13b
    c1f8:	0f 84 91 03 00 00    	je     c58f <py1_quicksort+0x68f>
    c1fe:	48 8d 05 2b 38 00 00 	lea    0x382b(%rip),%rax        # fa30 <pt_int_list_get>
    c205:	48 8b 54 24 60       	mov    0x60(%rsp),%rdx
    c20a:	4c 89 fe             	mov    %r15,%rsi
    c20d:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c212:	ff d0                	call   *%rax
    c214:	48 0f b6 4c 24 30    	movzbq 0x30(%rsp),%rcx
    c21a:	40 80 f9 01          	rex cmp $0x1,%cl
    c21e:	0f 84 29 03 00 00    	je     c54d <py1_quicksort+0x64d>
    c224:	45 84 f6             	test   %r14b,%r14b
    c227:	0f 84 08 03 00 00    	je     c535 <py1_quicksort+0x635>
    c22d:	48 8d 0d 7c 38 00 00 	lea    0x387c(%rip),%rcx        # fab0 <pt_int_list_set>
    c234:	48 89 c2             	mov    %rax,%rdx
    c237:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c23c:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c241:	ff d1                	call   *%rcx
    c243:	3c 01                	cmp    $0x1,%al
    c245:	0f 84 a5 02 00 00    	je     c4f0 <py1_quicksort+0x5f0>
    c24b:	45 84 ed             	test   %r13b,%r13b
    c24e:	0f 84 85 02 00 00    	je     c4d9 <py1_quicksort+0x5d9>
    c254:	48 8d 05 55 38 00 00 	lea    0x3855(%rip),%rax        # fab0 <pt_int_list_set>
    c25b:	48 89 da             	mov    %rbx,%rdx
    c25e:	4c 89 fe             	mov    %r15,%rsi
    c261:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c266:	ff d0                	call   *%rax
    c268:	3c 01                	cmp    $0x1,%al
    c26a:	0f 84 28 02 00 00    	je     c498 <py1_quicksort+0x598>
    c270:	45 84 f6             	test   %r14b,%r14b
    c273:	0f 84 08 02 00 00    	je     c481 <py1_quicksort+0x581>
    c279:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c27e:	48 83 c2 01          	add    $0x1,%rdx
    c282:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c287:	0f 90 c0             	seto   %al
    c28a:	84 c0                	test   %al,%al
    c28c:	0f 85 d8 01 00 00    	jne    c46a <py1_quicksort+0x56a>
    c292:	45 84 ed             	test   %r13b,%r13b
    c295:	0f 84 b8 01 00 00    	je     c453 <py1_quicksort+0x553>
    c29b:	49 83 ef 01          	sub    $0x1,%r15
    c29f:	0f 90 c0             	seto   %al
    c2a2:	84 c0                	test   %al,%al
    c2a4:	0f 85 92 01 00 00    	jne    c43c <py1_quicksort+0x53c>
    c2aa:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c2b0:	4d 89 f5             	mov    %r14,%r13
    c2b3:	48 8b 54 24 50       	mov    0x50(%rsp),%rdx
    c2b8:	4c 89 f9             	mov    %r15,%rcx
    c2bb:	4d 89 e7             	mov    %r12,%r15
    c2be:	e9 0a fd ff ff       	jmp    bfcd <py1_quicksort+0xcd>
    c2c3:	45 84 ed             	test   %r13b,%r13b
    c2c6:	0f 84 78 03 00 00    	je     c644 <py1_quicksort+0x744>
    c2cc:	49 83 ef 01          	sub    $0x1,%r15
    c2d0:	0f 90 c0             	seto   %al
    c2d3:	84 c0                	test   %al,%al
    c2d5:	0f 85 52 03 00 00    	jne    c62d <py1_quicksort+0x72d>
    c2db:	41 bd 01 00 00 00    	mov    $0x1,%r13d
    c2e1:	4c 89 7c 24 48       	mov    %r15,0x48(%rsp)
    c2e6:	e9 77 fe ff ff       	jmp    c162 <py1_quicksort+0x262>
    c2eb:	4c 8b 7c 24 48       	mov    0x48(%rsp),%r15
    c2f0:	45 84 f6             	test   %r14b,%r14b
    c2f3:	0f 84 d1 03 00 00    	je     c6ca <py1_quicksort+0x7ca>
    c2f9:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c2fe:	48 83 c2 01          	add    $0x1,%rdx
    c302:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c307:	0f 90 c0             	seto   %al
    c30a:	84 c0                	test   %al,%al
    c30c:	0f 85 a1 03 00 00    	jne    c6b3 <py1_quicksort+0x7b3>
    c312:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c318:	4c 89 7c 24 48       	mov    %r15,0x48(%rsp)
    c31d:	e9 fe fd ff ff       	jmp    c120 <py1_quicksort+0x220>
    c322:	48 8b 9c 24 80 00 00 	mov    0x80(%rsp),%rbx
    c329:	00 
    c32a:	4c 8b a4 24 88 00 00 	mov    0x88(%rsp),%r12
    c331:	00 
    c332:	4c 8b ac 24 90 00 00 	mov    0x90(%rsp),%r13
    c339:	00 
    c33a:	4c 8b b4 24 98 00 00 	mov    0x98(%rsp),%r14
    c341:	00 
    c342:	4c 8b bc 24 a0 00 00 	mov    0xa0(%rsp),%r15
    c349:	00 
    c34a:	48 81 c4 b0 00 00 00 	add    $0xb0,%rsp
    c351:	48 89 ec             	mov    %rbp,%rsp
    c354:	5d                   	pop    %rbp
    c355:	c3                   	ret
    c356:	48 8d 3d 40 53 04 00 	lea    0x45340(%rip),%rdi        # 5169d <.Ldata49>
    c35d:	be ca 00 00 00       	mov    $0xca,%esi
    c362:	48 8d 05 97 62 00 00 	lea    0x6297(%rip),%rax        # 12600 <pt_fail>
    c369:	ff d0                	call   *%rax
    c36b:	0f 0b                	ud2
    c36d:	48 8d 3d 6e 52 04 00 	lea    0x4526e(%rip),%rdi        # 515e2 <.Ldata48>
    c374:	be bb 00 00 00       	mov    $0xbb,%esi
    c379:	48 8d 05 80 62 00 00 	lea    0x6280(%rip),%rax        # 12600 <pt_fail>
    c380:	ff d0                	call   *%rax
    c382:	0f 0b                	ud2
    c384:	48 8d 3d 76 51 04 00 	lea    0x45176(%rip),%rdi        # 51501 <.Ldata47>
    c38b:	be e1 00 00 00       	mov    $0xe1,%esi
    c390:	48 8d 05 69 62 00 00 	lea    0x6269(%rip),%rax        # 12600 <pt_fail>
    c397:	ff d0                	call   *%rax
    c399:	0f 0b                	ud2
    c39b:	48 8d 3d 95 50 04 00 	lea    0x45095(%rip),%rdi        # 51437 <.Ldata46>
    c3a2:	be ca 00 00 00       	mov    $0xca,%esi
    c3a7:	48 8d 05 52 62 00 00 	lea    0x6252(%rip),%rax        # 12600 <pt_fail>
    c3ae:	ff d0                	call   *%rax
    c3b0:	0f 0b                	ud2
    c3b2:	48 8d 3d c3 4f 04 00 	lea    0x44fc3(%rip),%rdi        # 5137c <.Ldata45>
    c3b9:	be bb 00 00 00       	mov    $0xbb,%esi
    c3be:	48 8d 05 3b 62 00 00 	lea    0x623b(%rip),%rax        # 12600 <pt_fail>
    c3c5:	ff d0                	call   *%rax
    c3c7:	0f 0b                	ud2
    c3c9:	48 8d 3d c7 4e 04 00 	lea    0x44ec7(%rip),%rdi        # 51297 <.Ldata44>
    c3d0:	be e5 00 00 00       	mov    $0xe5,%esi
    c3d5:	48 8d 05 24 62 00 00 	lea    0x6224(%rip),%rax        # 12600 <pt_fail>
    c3dc:	ff d0                	call   *%rax
    c3de:	0f 0b                	ud2
    c3e0:	48 8d 3d 11 4e 04 00 	lea    0x44e11(%rip),%rdi        # 511f8 <.Ldata43>
    c3e7:	be 9f 00 00 00       	mov    $0x9f,%esi
    c3ec:	48 8d 05 0d 62 00 00 	lea    0x620d(%rip),%rax        # 12600 <pt_fail>
    c3f3:	ff d0                	call   *%rax
    c3f5:	0f 0b                	ud2
    c3f7:	48 8d 3d 1f 4d 04 00 	lea    0x44d1f(%rip),%rdi        # 5111d <.Ldata42>
    c3fe:	be db 00 00 00       	mov    $0xdb,%esi
    c403:	48 8d 05 f6 61 00 00 	lea    0x61f6(%rip),%rax        # 12600 <pt_fail>
    c40a:	ff d0                	call   *%rax
    c40c:	0f 0b                	ud2
    c40e:	48 8d 3d 72 4c 04 00 	lea    0x44c72(%rip),%rdi        # 51087 <.Ldata41>
    c415:	be 96 00 00 00       	mov    $0x96,%esi
    c41a:	48 8d 05 df 61 00 00 	lea    0x61df(%rip),%rax        # 12600 <pt_fail>
    c421:	ff d0                	call   *%rax
    c423:	0f 0b                	ud2
    c425:	48 8d 3d 8e 4b 04 00 	lea    0x44b8e(%rip),%rdi        # 50fba <.Ldata40>
    c42c:	be cd 00 00 00       	mov    $0xcd,%esi
    c431:	48 8d 05 c8 61 00 00 	lea    0x61c8(%rip),%rax        # 12600 <pt_fail>
    c438:	ff d0                	call   *%rax
    c43a:	0f 0b                	ud2
    c43c:	48 8d 3d e1 4a 04 00 	lea    0x44ae1(%rip),%rdi        # 50f24 <.Ldata3f>
    c443:	be 96 00 00 00       	mov    $0x96,%esi
    c448:	48 8d 05 b1 61 00 00 	lea    0x61b1(%rip),%rax        # 12600 <pt_fail>
    c44f:	ff d0                	call   *%rax
    c451:	0f 0b                	ud2
    c453:	48 8d 3d fd 49 04 00 	lea    0x449fd(%rip),%rdi        # 50e57 <.Ldata3e>
    c45a:	be cd 00 00 00       	mov    $0xcd,%esi
    c45f:	48 8d 05 9a 61 00 00 	lea    0x619a(%rip),%rax        # 12600 <pt_fail>
    c466:	ff d0                	call   *%rax
    c468:	0f 0b                	ud2
    c46a:	48 8d 3d 50 49 04 00 	lea    0x44950(%rip),%rdi        # 50dc1 <.Ldata3d>
    c471:	be 96 00 00 00       	mov    $0x96,%esi
    c476:	48 8d 05 83 61 00 00 	lea    0x6183(%rip),%rax        # 12600 <pt_fail>
    c47d:	ff d0                	call   *%rax
    c47f:	0f 0b                	ud2
    c481:	48 8d 3d 6c 48 04 00 	lea    0x4486c(%rip),%rdi        # 50cf4 <.Ldata3c>
    c488:	be cd 00 00 00       	mov    $0xcd,%esi
    c48d:	48 8d 05 6c 61 00 00 	lea    0x616c(%rip),%rax        # 12600 <pt_fail>
    c494:	ff d0                	call   *%rax
    c496:	0f 0b                	ud2
    c498:	b8 01 00 00 00       	mov    $0x1,%eax
    c49d:	48 8d 35 8d 45 04 00 	lea    0x4458d(%rip),%rsi        # 50a31 <.Ldata35>
    c4a4:	ba 22 00 00 00       	mov    $0x22,%edx
    c4a9:	48 8d 0d cb 47 04 00 	lea    0x447cb(%rip),%rcx        # 50c7b <.Ldata3a>
    c4b0:	41 b8 44 00 00 00    	mov    $0x44,%r8d
    c4b6:	4c 8d 0d 02 48 04 00 	lea    0x44802(%rip),%r9        # 50cbf <.Ldata3b>
    c4bd:	41 ba 35 00 00 00    	mov    $0x35,%r10d
    c4c3:	48 0f b6 f8          	movzbq %al,%rdi
    c4c7:	4c 89 14 24          	mov    %r10,(%rsp)
    c4cb:	48 8d 05 5e 6a 00 00 	lea    0x6a5e(%rip),%rax        # 12f30 <pt_raise_msg>
    c4d2:	ff d0                	call   *%rax
    c4d4:	e9 49 fe ff ff       	jmp    c322 <py1_quicksort+0x422>
    c4d9:	48 8d 3d c8 46 04 00 	lea    0x446c8(%rip),%rdi        # 50ba8 <.Ldata39>
    c4e0:	be d3 00 00 00       	mov    $0xd3,%esi
    c4e5:	48 8d 05 14 61 00 00 	lea    0x6114(%rip),%rax        # 12600 <pt_fail>
    c4ec:	ff d0                	call   *%rax
    c4ee:	0f 0b                	ud2
    c4f0:	b9 01 00 00 00       	mov    $0x1,%ecx
    c4f5:	48 8d 35 35 45 04 00 	lea    0x44535(%rip),%rsi        # 50a31 <.Ldata35>
    c4fc:	ba 22 00 00 00       	mov    $0x22,%edx
    c501:	4c 8d 15 4b 45 04 00 	lea    0x4454b(%rip),%r10        # 50a53 <.Ldata36>
    c508:	41 b8 44 00 00 00    	mov    $0x44,%r8d
    c50e:	4c 8d 0d 82 45 04 00 	lea    0x44582(%rip),%r9        # 50a97 <.Ldata37>
    c515:	41 bb 39 00 00 00    	mov    $0x39,%r11d
    c51b:	48 0f b6 f9          	movzbq %cl,%rdi
    c51f:	4c 89 1c 24          	mov    %r11,(%rsp)
    c523:	4c 8d 1d 06 6a 00 00 	lea    0x6a06(%rip),%r11        # 12f30 <pt_raise_msg>
    c52a:	4c 89 d1             	mov    %r10,%rcx
    c52d:	41 ff d3             	call   *%r11
    c530:	e9 ed fd ff ff       	jmp    c322 <py1_quicksort+0x422>
    c535:	48 8d 3d 1e 44 04 00 	lea    0x4441e(%rip),%rdi        # 5095a <.Ldata34>
    c53c:	be d7 00 00 00       	mov    $0xd7,%esi
    c541:	4c 8d 05 b8 60 00 00 	lea    0x60b8(%rip),%r8        # 12600 <pt_fail>
    c548:	41 ff d0             	call   *%r8
    c54b:	0f 0b                	ud2
    c54d:	bf 01 00 00 00       	mov    $0x1,%edi
    c552:	48 8d 35 fd 2f 04 00 	lea    0x42ffd(%rip),%rsi        # 4f556 <.Ldatac>
    c559:	ba 17 00 00 00       	mov    $0x17,%edx
    c55e:	48 8d 0d 70 43 04 00 	lea    0x44370(%rip),%rcx        # 508d5 <.Ldata32>
    c565:	41 b8 44 00 00 00    	mov    $0x44,%r8d
    c56b:	4c 8d 0d a7 43 04 00 	lea    0x443a7(%rip),%r9        # 50919 <.Ldata33>
    c572:	41 ba 41 00 00 00    	mov    $0x41,%r10d
    c578:	48 0f b6 ff          	movzbq %dil,%rdi
    c57c:	4c 89 14 24          	mov    %r10,(%rsp)
    c580:	4c 8d 15 a9 69 00 00 	lea    0x69a9(%rip),%r10        # 12f30 <pt_raise_msg>
    c587:	41 ff d2             	call   *%r10
    c58a:	e9 93 fd ff ff       	jmp    c322 <py1_quicksort+0x422>
    c58f:	48 8d 3d 60 42 04 00 	lea    0x44260(%rip),%rdi        # 507f6 <.Ldata31>
    c596:	be df 00 00 00       	mov    $0xdf,%esi
    c59b:	4c 8d 1d 5e 60 00 00 	lea    0x605e(%rip),%r11        # 12600 <pt_fail>
    c5a2:	41 ff d3             	call   *%r11
    c5a5:	0f 0b                	ud2
    c5a7:	b8 01 00 00 00       	mov    $0x1,%eax
    c5ac:	48 8d 35 a3 2f 04 00 	lea    0x42fa3(%rip),%rsi        # 4f556 <.Ldatac>
    c5b3:	ba 17 00 00 00       	mov    $0x17,%edx
    c5b8:	48 8d 0d ba 41 04 00 	lea    0x441ba(%rip),%rcx        # 50779 <.Ldata2f>
    c5bf:	41 b8 44 00 00 00    	mov    $0x44,%r8d
    c5c5:	4c 8d 0d f1 41 04 00 	lea    0x441f1(%rip),%r9        # 507bd <.Ldata30>
    c5cc:	41 ba 39 00 00 00    	mov    $0x39,%r10d
    c5d2:	48 0f b6 f8          	movzbq %al,%rdi
    c5d6:	4c 89 14 24          	mov    %r10,(%rsp)
    c5da:	48 8d 05 4f 69 00 00 	lea    0x694f(%rip),%rax        # 12f30 <pt_raise_msg>
    c5e1:	ff d0                	call   *%rax
    c5e3:	e9 3a fd ff ff       	jmp    c322 <py1_quicksort+0x422>
    c5e8:	48 8d 3d b3 40 04 00 	lea    0x440b3(%rip),%rdi        # 506a2 <.Ldata2e>
    c5ef:	be d7 00 00 00       	mov    $0xd7,%esi
    c5f4:	48 8d 05 05 60 00 00 	lea    0x6005(%rip),%rax        # 12600 <pt_fail>
    c5fb:	ff d0                	call   *%rax
    c5fd:	0f 0b                	ud2
    c5ff:	48 8d 3d cb 3f 04 00 	lea    0x43fcb(%rip),%rdi        # 505d1 <.Ldata2d>
    c606:	be d1 00 00 00       	mov    $0xd1,%esi
    c60b:	48 8d 05 ee 5f 00 00 	lea    0x5fee(%rip),%rax        # 12600 <pt_fail>
    c612:	ff d0                	call   *%rax
    c614:	0f 0b                	ud2
    c616:	48 8d 3d e8 3e 04 00 	lea    0x43ee8(%rip),%rdi        # 50505 <.Ldata2c>
    c61d:	be cc 00 00 00       	mov    $0xcc,%esi
    c622:	48 8d 05 d7 5f 00 00 	lea    0x5fd7(%rip),%rax        # 12600 <pt_fail>
    c629:	ff d0                	call   *%rax
    c62b:	0f 0b                	ud2
    c62d:	48 8d 3d 3b 3e 04 00 	lea    0x43e3b(%rip),%rdi        # 5046f <.Ldata2b>
    c634:	be 96 00 00 00       	mov    $0x96,%esi
    c639:	48 8d 05 c0 5f 00 00 	lea    0x5fc0(%rip),%rax        # 12600 <pt_fail>
    c640:	ff d0                	call   *%rax
    c642:	0f 0b                	ud2
    c644:	48 8d 3d 57 3d 04 00 	lea    0x43d57(%rip),%rdi        # 503a2 <.Ldata2a>
    c64b:	be cd 00 00 00       	mov    $0xcd,%esi
    c650:	48 8d 05 a9 5f 00 00 	lea    0x5fa9(%rip),%rax        # 12600 <pt_fail>
    c657:	ff d0                	call   *%rax
    c659:	0f 0b                	ud2
    c65b:	b8 01 00 00 00       	mov    $0x1,%eax
    c660:	48 8d 35 ef 2e 04 00 	lea    0x42eef(%rip),%rsi        # 4f556 <.Ldatac>
    c667:	ba 17 00 00 00       	mov    $0x17,%edx
    c66c:	48 8d 0d c4 3b 04 00 	lea    0x43bc4(%rip),%rcx        # 50237 <.Ldata27>
    c673:	41 b8 44 00 00 00    	mov    $0x44,%r8d
    c679:	4c 8d 0d fb 3b 04 00 	lea    0x43bfb(%rip),%r9        # 5027b <.Ldata28>
    c680:	41 ba 3e 00 00 00    	mov    $0x3e,%r10d
    c686:	48 0f b6 f8          	movzbq %al,%rdi
    c68a:	4c 89 14 24          	mov    %r10,(%rsp)
    c68e:	48 8d 05 9b 68 00 00 	lea    0x689b(%rip),%rax        # 12f30 <pt_raise_msg>
    c695:	ff d0                	call   *%rax
    c697:	e9 86 fc ff ff       	jmp    c322 <py1_quicksort+0x422>
    c69c:	48 8d 3d b8 3a 04 00 	lea    0x43ab8(%rip),%rdi        # 5015b <.Ldata26>
    c6a3:	be dc 00 00 00       	mov    $0xdc,%esi
    c6a8:	48 8d 05 51 5f 00 00 	lea    0x5f51(%rip),%rax        # 12600 <pt_fail>
    c6af:	ff d0                	call   *%rax
    c6b1:	0f 0b                	ud2
    c6b3:	48 8d 3d 0c 3a 04 00 	lea    0x43a0c(%rip),%rdi        # 500c6 <.Ldata25>
    c6ba:	be 95 00 00 00       	mov    $0x95,%esi
    c6bf:	48 8d 05 3a 5f 00 00 	lea    0x5f3a(%rip),%rax        # 12600 <pt_fail>
    c6c6:	ff d0                	call   *%rax
    c6c8:	0f 0b                	ud2
    c6ca:	48 8d 3d 29 39 04 00 	lea    0x43929(%rip),%rdi        # 4fffa <.Ldata24>
    c6d1:	be cc 00 00 00       	mov    $0xcc,%esi
    c6d6:	48 8d 05 23 5f 00 00 	lea    0x5f23(%rip),%rax        # 12600 <pt_fail>
    c6dd:	ff d0                	call   *%rax
    c6df:	0f 0b                	ud2
    c6e1:	b8 01 00 00 00       	mov    $0x1,%eax
    c6e6:	48 8d 35 69 2e 04 00 	lea    0x42e69(%rip),%rsi        # 4f556 <.Ldatac>
    c6ed:	ba 17 00 00 00       	mov    $0x17,%edx
    c6f2:	48 8d 0d 98 37 04 00 	lea    0x43798(%rip),%rcx        # 4fe91 <.Ldata21>
    c6f9:	41 b8 43 00 00 00    	mov    $0x43,%r8d
    c6ff:	4c 8d 0d ce 37 04 00 	lea    0x437ce(%rip),%r9        # 4fed4 <.Ldata22>
    c706:	41 ba 3e 00 00 00    	mov    $0x3e,%r10d
    c70c:	48 0f b6 f8          	movzbq %al,%rdi
    c710:	4c 89 14 24          	mov    %r10,(%rsp)
    c714:	48 8d 05 15 68 00 00 	lea    0x6815(%rip),%rax        # 12f30 <pt_raise_msg>
    c71b:	ff d0                	call   *%rax
    c71d:	e9 00 fc ff ff       	jmp    c322 <py1_quicksort+0x422>
    c722:	48 8d 3d 8d 36 04 00 	lea    0x4368d(%rip),%rdi        # 4fdb6 <.Ldata20>
    c729:	be db 00 00 00       	mov    $0xdb,%esi
    c72e:	48 8d 05 cb 5e 00 00 	lea    0x5ecb(%rip),%rax        # 12600 <pt_fail>
    c735:	ff d0                	call   *%rax
    c737:	0f 0b                	ud2
    c739:	48 8d 3d a8 35 04 00 	lea    0x435a8(%rip),%rdi        # 4fce8 <.Ldata1f>
    c740:	be ce 00 00 00       	mov    $0xce,%esi
    c745:	48 8d 05 b4 5e 00 00 	lea    0x5eb4(%rip),%rax        # 12600 <pt_fail>
    c74c:	ff d0                	call   *%rax
    c74e:	0f 0b                	ud2
    c750:	48 8d 3d c8 34 04 00 	lea    0x434c8(%rip),%rdi        # 4fc1f <.Ldata1e>
    c757:	be c9 00 00 00       	mov    $0xc9,%esi
    c75c:	48 8d 05 9d 5e 00 00 	lea    0x5e9d(%rip),%rax        # 12600 <pt_fail>
    c763:	ff d0                	call   *%rax
    c765:	0f 0b                	ud2
    c767:	b8 01 00 00 00       	mov    $0x1,%eax
    c76c:	48 8d 35 e3 2d 04 00 	lea    0x42de3(%rip),%rsi        # 4f556 <.Ldatac>
    c773:	ba 17 00 00 00       	mov    $0x17,%edx
    c778:	48 8d 0d 12 34 04 00 	lea    0x43412(%rip),%rcx        # 4fb91 <.Ldata1c>
    c77f:	41 b8 43 00 00 00    	mov    $0x43,%r8d
    c785:	4c 8d 0d 48 34 04 00 	lea    0x43448(%rip),%r9        # 4fbd4 <.Ldata1d>
    c78c:	41 ba 4b 00 00 00    	mov    $0x4b,%r10d
    c792:	48 0f b6 f8          	movzbq %al,%rdi
    c796:	4c 89 14 24          	mov    %r10,(%rsp)
    c79a:	48 8d 05 8f 67 00 00 	lea    0x678f(%rip),%rax        # 12f30 <pt_raise_msg>
    c7a1:	ff d0                	call   *%rax
    c7a3:	e9 7a fb ff ff       	jmp    c322 <py1_quicksort+0x422>
    c7a8:	48 8d 3d 3b 33 04 00 	lea    0x4333b(%rip),%rdi        # 4faea <.Ldata1b>
    c7af:	be a7 00 00 00       	mov    $0xa7,%esi
    c7b4:	48 8d 05 45 5e 00 00 	lea    0x5e45(%rip),%rax        # 12600 <pt_fail>
    c7bb:	ff d0                	call   *%rax
    c7bd:	0f 0b                	ud2

Disassembly of section .fini:
