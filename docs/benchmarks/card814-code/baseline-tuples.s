
/workspace/scratch/2ce9dd46a989/card814-base/tuples:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000bda7 <pt_scalar_pair_1>:
    bda7:	55                   	push   %rbp
    bda8:	48 89 e5             	mov    %rsp,%rbp
    bdab:	48 83 ec 20          	sub    $0x20,%rsp
    bdaf:	48 89 1c 24          	mov    %rbx,(%rsp)
    bdb3:	4c 89 64 24 08       	mov    %r12,0x8(%rsp)
    bdb8:	4c 89 6c 24 10       	mov    %r13,0x10(%rsp)
    bdbd:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    bdc2:	48 33 c0             	xor    %rax,%rax
    bdc5:	49 89 c6             	mov    %rax,%r14
    bdc8:	4d 89 f5             	mov    %r14,%r13
    bdcb:	48 85 f6             	test   %rsi,%rsi
    bdce:	0f 8f 0b 00 00 00    	jg     bddf <pt_scalar_pair_1+0x38>
    bdd4:	4c 89 f0             	mov    %r14,%rax
    bdd7:	4c 89 ea             	mov    %r13,%rdx
    bdda:	e9 77 00 00 00       	jmp    be56 <pt_scalar_pair_1+0xaf>
    bddf:	48 89 fb             	mov    %rdi,%rbx
    bde2:	4c 8b 63 08          	mov    0x8(%rbx),%r12
    bde6:	49 81 fc e8 03 00 00 	cmp    $0x3e8,%r12
    bded:	0f 8d ad 00 00 00    	jge    bea0 <pt_scalar_pair_1+0xf9>
    bdf3:	49 8d 4c 24 01       	lea    0x1(%r12),%rcx
    bdf8:	48 89 4b 08          	mov    %rcx,0x8(%rbx)
    bdfc:	ba 0a 00 00 00       	mov    $0xa,%edx
    be01:	48 89 df             	mov    %rbx,%rdi
    be04:	e8 d7 02 00 00       	call   c0e0 <pt_scalar_pair_2>
    be09:	48 89 c6             	mov    %rax,%rsi
    be0c:	4c 89 63 08          	mov    %r12,0x8(%rbx)
    be10:	48 0f b6 03          	movzbq (%rbx),%rax
    be14:	84 c0                	test   %al,%al
    be16:	0f 85 34 00 00 00    	jne    be50 <pt_scalar_pair_1+0xa9>
    be1c:	4c 89 f0             	mov    %r14,%rax
    be1f:	48 83 c0 01          	add    $0x1,%rax
    be23:	0f 90 c1             	seto   %cl
    be26:	84 c9                	test   %cl,%cl
    be28:	0f 85 5b 00 00 00    	jne    be89 <pt_scalar_pair_1+0xe2>
    be2e:	48 89 d1             	mov    %rdx,%rcx
    be31:	4c 89 ea             	mov    %r13,%rdx
    be34:	48 03 d1             	add    %rcx,%rdx
    be37:	0f 90 c1             	seto   %cl
    be3a:	84 c9                	test   %cl,%cl
    be3c:	0f 85 30 00 00 00    	jne    be72 <pt_scalar_pair_1+0xcb>
    be42:	48 89 df             	mov    %rbx,%rdi
    be45:	49 89 d5             	mov    %rdx,%r13
    be48:	49 89 c6             	mov    %rax,%r14
    be4b:	e9 7b ff ff ff       	jmp    bdcb <pt_scalar_pair_1+0x24>
    be50:	48 33 c0             	xor    %rax,%rax
    be53:	48 89 c2             	mov    %rax,%rdx
    be56:	48 8b 1c 24          	mov    (%rsp),%rbx
    be5a:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
    be5f:	4c 8b 6c 24 10       	mov    0x10(%rsp),%r13
    be64:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
    be69:	48 83 c4 20          	add    $0x20,%rsp
    be6d:	48 89 ec             	mov    %rbp,%rsp
    be70:	5d                   	pop    %rbp
    be71:	c3                   	ret
    be72:	48 8d 3d 5f 05 04 00 	lea    0x4055f(%rip),%rdi        # 4c3d8 <.Ldata7>
    be79:	be 97 00 00 00       	mov    $0x97,%esi
    be7e:	48 8d 05 fb 59 00 00 	lea    0x59fb(%rip),%rax        # 11880 <pt_fail>
    be85:	ff d0                	call   *%rax
    be87:	0f 0b                	ud2
    be89:	48 8d 3d b1 04 04 00 	lea    0x404b1(%rip),%rdi        # 4c341 <.Ldata6>
    be90:	be 97 00 00 00       	mov    $0x97,%esi
    be95:	48 8d 05 e4 59 00 00 	lea    0x59e4(%rip),%rax        # 11880 <pt_fail>
    be9c:	ff d0                	call   *%rax
    be9e:	0f 0b                	ud2
    bea0:	48 8d 3d d7 03 04 00 	lea    0x403d7(%rip),%rdi        # 4c27e <.Ldata4>
    bea7:	be bf 00 00 00       	mov    $0xbf,%esi
    beac:	48 8d 05 cd 59 00 00 	lea    0x59cd(%rip),%rax        # 11880 <pt_fail>
    beb3:	ff d0                	call   *%rax
    beb5:	0f 0b                	ud2

Disassembly of section .fini:

/workspace/scratch/2ce9dd46a989/card814-base/tuples:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000c0e0 <pt_scalar_pair_2>:
    c0e0:	55                   	push   %rbp
    c0e1:	48 89 e5             	mov    %rsp,%rbp
    c0e4:	48 83 ec 10          	sub    $0x10,%rsp
    c0e8:	48 85 d2             	test   %rdx,%rdx
    c0eb:	0f 84 3b 01 00 00    	je     c22c <pt_scalar_pair_2+0x14c>
    c0f1:	48 3b 35 80 01 00 00 	cmp    0x180(%rip),%rsi        # c278 <pt_scalar_pair_2+0x198>
    c0f8:	0f 94 c0             	sete   %al
    c0fb:	48 83 fa ff          	cmp    $0xffffffffffffffff,%rdx
    c0ff:	49 89 d2             	mov    %rdx,%r10
    c102:	0f 94 c1             	sete   %cl
    c105:	84 c1                	test   %al,%cl
    c107:	0f 85 07 01 00 00    	jne    c214 <pt_scalar_pair_2+0x134>
    c10d:	48 89 f0             	mov    %rsi,%rax
    c110:	49 89 f1             	mov    %rsi,%r9
    c113:	48 99                	cqto
    c115:	4c 89 d1             	mov    %r10,%rcx
    c118:	48 85 c9             	test   %rcx,%rcx
    c11b:	0f 84 4e 01 00 00    	je     c26f <pt_scalar_pair_2+0x18f>
    c121:	4c 89 c8             	mov    %r9,%rax
    c124:	48 f7 f9             	idiv   %rcx
    c127:	49 89 c0             	mov    %rax,%r8
    c12a:	4c 89 c8             	mov    %r9,%rax
    c12d:	48 99                	cqto
    c12f:	48 83 f9 ff          	cmp    $0xffffffffffffffff,%rcx
    c133:	0f 85 0a 00 00 00    	jne    c143 <pt_scalar_pair_2+0x63>
    c139:	ba 00 00 00 00       	mov    $0x0,%edx
    c13e:	e9 03 00 00 00       	jmp    c146 <pt_scalar_pair_2+0x66>
    c143:	48 f7 f9             	idiv   %rcx
    c146:	48 89 d7             	mov    %rdx,%rdi
    c149:	4d 85 d2             	test   %r10,%r10
    c14c:	0f 84 80 00 00 00    	je     c1d2 <pt_scalar_pair_2+0xf2>
    c152:	49 83 fa ff          	cmp    $0xffffffffffffffff,%r10
    c156:	4c 89 d6             	mov    %r10,%rsi
    c159:	48 0f 44 35 1f 01 00 	cmove  0x11f(%rip),%rsi        # c280 <pt_scalar_pair_2+0x1a0>
    c160:	00 
    c161:	4c 89 c8             	mov    %r9,%rax
    c164:	48 99                	cqto
    c166:	48 83 fe ff          	cmp    $0xffffffffffffffff,%rsi
    c16a:	0f 85 0a 00 00 00    	jne    c17a <pt_scalar_pair_2+0x9a>
    c170:	ba 00 00 00 00       	mov    $0x0,%edx
    c175:	e9 03 00 00 00       	jmp    c17d <pt_scalar_pair_2+0x9d>
    c17a:	48 f7 fe             	idiv   %rsi
    c17d:	49 89 d1             	mov    %rdx,%r9
    c180:	48 33 c9             	xor    %rcx,%rcx
    c183:	48 89 f8             	mov    %rdi,%rax
    c186:	48 85 c0             	test   %rax,%rax
    c189:	40 0f 95 c7          	setne  %dil
    c18d:	49 33 c2             	xor    %r10,%rax
    c190:	48 c1 e8 3f          	shr    $0x3f,%rax
    c194:	23 f8                	and    %eax,%edi
    c196:	48 0f b6 d7          	movzbq %dil,%rdx
    c19a:	4c 89 c0             	mov    %r8,%rax
    c19d:	48 2b c2             	sub    %rdx,%rax
    c1a0:	4c 89 ca             	mov    %r9,%rdx
    c1a3:	48 85 d2             	test   %rdx,%rdx
    c1a6:	40 0f 95 c7          	setne  %dil
    c1aa:	49 89 d0             	mov    %rdx,%r8
    c1ad:	4c 33 c6             	xor    %rsi,%r8
    c1b0:	49 c1 e8 3f          	shr    $0x3f,%r8
    c1b4:	41 84 f8             	test   %dil,%r8b
    c1b7:	48 0f 45 ce          	cmovne %rsi,%rcx
    c1bb:	48 03 d1             	add    %rcx,%rdx
    c1be:	e9 06 00 00 00       	jmp    c1c9 <pt_scalar_pair_2+0xe9>
    c1c3:	48 33 c0             	xor    %rax,%rax
    c1c6:	48 89 c2             	mov    %rax,%rdx
    c1c9:	48 83 c4 10          	add    $0x10,%rsp
    c1cd:	48 89 ec             	mov    %rbp,%rsp
    c1d0:	5d                   	pop    %rbp
    c1d1:	c3                   	ret
    c1d2:	bf 02 00 00 00       	mov    $0x2,%edi
    c1d7:	48 8d 35 91 02 04 00 	lea    0x40291(%rip),%rsi        # 4c46f <.Ldata8>
    c1de:	ba 22 00 00 00       	mov    $0x22,%edx
    c1e3:	48 8d 0d c6 03 04 00 	lea    0x403c6(%rip),%rcx        # 4c5b0 <.Ldatac>
    c1ea:	41 b8 4d 00 00 00    	mov    $0x4d,%r8d
    c1f0:	4c 8d 0d 06 04 04 00 	lea    0x40406(%rip),%r9        # 4c5fd <.Ldatad>
    c1f7:	41 ba 3a 00 00 00    	mov    $0x3a,%r10d
    c1fd:	48 0f b6 ff          	movzbq %dil,%rdi
    c201:	4c 89 14 24          	mov    %r10,(%rsp)
    c205:	4c 8d 15 34 5e 00 00 	lea    0x5e34(%rip),%r10        # 12040 <pt_raise_msg>
    c20c:	41 ff d2             	call   *%r10
    c20f:	e9 af ff ff ff       	jmp    c1c3 <pt_scalar_pair_2+0xe3>
    c214:	48 8d 3d f6 02 04 00 	lea    0x402f6(%rip),%rdi        # 4c511 <.Ldatab>
    c21b:	be 9f 00 00 00       	mov    $0x9f,%esi
    c220:	4c 8d 05 59 56 00 00 	lea    0x5659(%rip),%r8        # 11880 <pt_fail>
    c227:	41 ff d0             	call   *%r8
    c22a:	0f 0b                	ud2
    c22c:	41 ba 02 00 00 00    	mov    $0x2,%r10d
    c232:	48 8d 35 36 02 04 00 	lea    0x40236(%rip),%rsi        # 4c46f <.Ldata8>
    c239:	ba 22 00 00 00       	mov    $0x22,%edx
    c23e:	48 8d 0d 4c 02 04 00 	lea    0x4024c(%rip),%rcx        # 4c491 <.Ldata9>
    c245:	41 b8 4d 00 00 00    	mov    $0x4d,%r8d
    c24b:	4c 8d 0d 8c 02 04 00 	lea    0x4028c(%rip),%r9        # 4c4de <.Ldataa>
    c252:	41 bb 33 00 00 00    	mov    $0x33,%r11d
    c258:	49 0f b6 fa          	movzbq %r10b,%rdi
    c25c:	4c 89 1c 24          	mov    %r11,(%rsp)
    c260:	4c 8d 1d d9 5d 00 00 	lea    0x5dd9(%rip),%r11        # 12040 <pt_raise_msg>
    c267:	41 ff d3             	call   *%r11
    c26a:	e9 54 ff ff ff       	jmp    c1c3 <pt_scalar_pair_2+0xe3>
    c26f:	0f 0b                	ud2
	...
    c27d:	00 00                	add    %al,(%rax)
    c27f:	80 01 00             	addb   $0x0,(%rcx)
    c282:	00 00                	add    %al,(%rax)
    c284:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:

/workspace/scratch/2ce9dd46a989/card814-base/tuples:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000bc5f <py1_digits>:
    bc5f:	55                   	push   %rbp
    bc60:	48 89 e5             	mov    %rsp,%rbp
    bc63:	48 83 ec 20          	sub    $0x20,%rsp
    bc67:	48 89 1c 24          	mov    %rbx,(%rsp)
    bc6b:	4c 89 64 24 08       	mov    %r12,0x8(%rsp)
    bc70:	4c 89 6c 24 10       	mov    %r13,0x10(%rsp)
    bc75:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    bc7a:	48 33 c0             	xor    %rax,%rax
    bc7d:	49 89 c5             	mov    %rax,%r13
    bc80:	4d 89 ee             	mov    %r13,%r14
    bc83:	48 85 f6             	test   %rsi,%rsi
    bc86:	0f 8f 43 00 00 00    	jg     bccf <py1_digits+0x70>
    bc8c:	48 8d 3d aa 06 04 00 	lea    0x406aa(%rip),%rdi        # 4c33d <.Ldata5>
    bc93:	be 04 00 00 00       	mov    $0x4,%esi
    bc98:	ba 02 00 00 00       	mov    $0x2,%edx
    bc9d:	48 8d 05 3c 38 00 00 	lea    0x383c(%rip),%rax        # f4e0 <pt_seq_new>
    bca4:	ff d0                	call   *%rax
    bca6:	48 89 c3             	mov    %rax,%rbx
    bca9:	48 8d 05 00 39 00 00 	lea    0x3900(%rip),%rax        # f5b0 <pt_seq_push>
    bcb0:	4c 89 ee             	mov    %r13,%rsi
    bcb3:	48 89 df             	mov    %rbx,%rdi
    bcb6:	ff d0                	call   *%rax
    bcb8:	48 8d 05 f1 38 00 00 	lea    0x38f1(%rip),%rax        # f5b0 <pt_seq_push>
    bcbf:	4c 89 f6             	mov    %r14,%rsi
    bcc2:	48 89 df             	mov    %rbx,%rdi
    bcc5:	ff d0                	call   *%rax
    bcc7:	48 89 d8             	mov    %rbx,%rax
    bcca:	e9 77 00 00 00       	jmp    bd46 <py1_digits+0xe7>
    bccf:	49 89 fc             	mov    %rdi,%r12
    bcd2:	49 8b 5c 24 08       	mov    0x8(%r12),%rbx
    bcd7:	48 81 fb e8 03 00 00 	cmp    $0x3e8,%rbx
    bcde:	0f 8d ac 00 00 00    	jge    bd90 <py1_digits+0x131>
    bce4:	48 8d 4b 01          	lea    0x1(%rbx),%rcx
    bce8:	49 89 4c 24 08       	mov    %rcx,0x8(%r12)
    bced:	ba 0a 00 00 00       	mov    $0xa,%edx
    bcf2:	4c 89 e7             	mov    %r12,%rdi
    bcf5:	e8 e6 03 00 00       	call   c0e0 <pt_scalar_pair_2>
    bcfa:	48 89 c7             	mov    %rax,%rdi
    bcfd:	49 89 5c 24 08       	mov    %rbx,0x8(%r12)
    bd02:	49 0f b6 04 24       	movzbq (%r12),%rax
    bd07:	84 c0                	test   %al,%al
    bd09:	0f 85 34 00 00 00    	jne    bd43 <py1_digits+0xe4>
    bd0f:	4c 89 ee             	mov    %r13,%rsi
    bd12:	48 83 c6 01          	add    $0x1,%rsi
    bd16:	0f 90 c0             	seto   %al
    bd19:	84 c0                	test   %al,%al
    bd1b:	0f 85 58 00 00 00    	jne    bd79 <py1_digits+0x11a>
    bd21:	4c 89 f1             	mov    %r14,%rcx
    bd24:	48 03 ca             	add    %rdx,%rcx
    bd27:	0f 90 c0             	seto   %al
    bd2a:	84 c0                	test   %al,%al
    bd2c:	0f 85 30 00 00 00    	jne    bd62 <py1_digits+0x103>
    bd32:	49 89 ce             	mov    %rcx,%r14
    bd35:	49 89 f5             	mov    %rsi,%r13
    bd38:	48 89 fe             	mov    %rdi,%rsi
    bd3b:	4c 89 e7             	mov    %r12,%rdi
    bd3e:	e9 40 ff ff ff       	jmp    bc83 <py1_digits+0x24>
    bd43:	48 33 c0             	xor    %rax,%rax
    bd46:	48 8b 1c 24          	mov    (%rsp),%rbx
    bd4a:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
    bd4f:	4c 8b 6c 24 10       	mov    0x10(%rsp),%r13
    bd54:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
    bd59:	48 83 c4 20          	add    $0x20,%rsp
    bd5d:	48 89 ec             	mov    %rbp,%rsp
    bd60:	5d                   	pop    %rbp
    bd61:	c3                   	ret
    bd62:	48 8d 3d 6f 06 04 00 	lea    0x4066f(%rip),%rdi        # 4c3d8 <.Ldata7>
    bd69:	be 97 00 00 00       	mov    $0x97,%esi
    bd6e:	48 8d 05 0b 5b 00 00 	lea    0x5b0b(%rip),%rax        # 11880 <pt_fail>
    bd75:	ff d0                	call   *%rax
    bd77:	0f 0b                	ud2
    bd79:	48 8d 3d c1 05 04 00 	lea    0x405c1(%rip),%rdi        # 4c341 <.Ldata6>
    bd80:	be 97 00 00 00       	mov    $0x97,%esi
    bd85:	48 8d 05 f4 5a 00 00 	lea    0x5af4(%rip),%rax        # 11880 <pt_fail>
    bd8c:	ff d0                	call   *%rax
    bd8e:	0f 0b                	ud2
    bd90:	48 8d 3d e7 04 04 00 	lea    0x404e7(%rip),%rdi        # 4c27e <.Ldata4>
    bd97:	be bf 00 00 00       	mov    $0xbf,%esi
    bd9c:	48 8d 05 dd 5a 00 00 	lea    0x5add(%rip),%rax        # 11880 <pt_fail>
    bda3:	ff d0                	call   *%rax
    bda5:	0f 0b                	ud2

Disassembly of section .fini:

/workspace/scratch/2ce9dd46a989/card814-base/tuples:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000beb8 <py2_div_mod>:
    beb8:	55                   	push   %rbp
    beb9:	48 89 e5             	mov    %rsp,%rbp
    bebc:	48 83 ec 50          	sub    $0x50,%rsp
    bec0:	48 89 5c 24 20       	mov    %rbx,0x20(%rsp)
    bec5:	4c 89 64 24 28       	mov    %r12,0x28(%rsp)
    beca:	4c 89 6c 24 30       	mov    %r13,0x30(%rsp)
    becf:	4c 89 74 24 38       	mov    %r14,0x38(%rsp)
    bed4:	4c 89 7c 24 40       	mov    %r15,0x40(%rsp)
    bed9:	48 85 d2             	test   %rdx,%rdx
    bedc:	0f 84 a8 01 00 00    	je     c08a <py2_div_mod+0x1d2>
    bee2:	48 3b 35 e7 01 00 00 	cmp    0x1e7(%rip),%rsi        # c0d0 <py2_div_mod+0x218>
    bee9:	0f 94 c0             	sete   %al
    beec:	48 83 fa ff          	cmp    $0xffffffffffffffff,%rdx
    bef0:	49 89 d5             	mov    %rdx,%r13
    bef3:	0f 94 c1             	sete   %cl
    bef6:	84 c1                	test   %al,%cl
    bef8:	0f 85 75 01 00 00    	jne    c073 <py2_div_mod+0x1bb>
    befe:	48 89 f0             	mov    %rsi,%rax
    bf01:	49 89 f0             	mov    %rsi,%r8
    bf04:	48 99                	cqto
    bf06:	4c 89 e9             	mov    %r13,%rcx
    bf09:	48 85 c9             	test   %rcx,%rcx
    bf0c:	0f 84 b9 01 00 00    	je     c0cb <py2_div_mod+0x213>
    bf12:	4c 89 c0             	mov    %r8,%rax
    bf15:	48 f7 f9             	idiv   %rcx
    bf18:	49 89 c7             	mov    %rax,%r15
    bf1b:	4c 89 c0             	mov    %r8,%rax
    bf1e:	48 99                	cqto
    bf20:	48 83 f9 ff          	cmp    $0xffffffffffffffff,%rcx
    bf24:	0f 85 0a 00 00 00    	jne    bf34 <py2_div_mod+0x7c>
    bf2a:	ba 00 00 00 00       	mov    $0x0,%edx
    bf2f:	e9 03 00 00 00       	jmp    bf37 <py2_div_mod+0x7f>
    bf34:	48 f7 f9             	idiv   %rcx
    bf37:	48 89 54 24 10       	mov    %rdx,0x10(%rsp)
    bf3c:	48 85 c9             	test   %rcx,%rcx
    bf3f:	0f 84 ed 00 00 00    	je     c032 <py2_div_mod+0x17a>
    bf45:	48 83 f9 ff          	cmp    $0xffffffffffffffff,%rcx
    bf49:	49 89 cc             	mov    %rcx,%r12
    bf4c:	4c 0f 44 25 84 01 00 	cmove  0x184(%rip),%r12        # c0d8 <py2_div_mod+0x220>
    bf53:	00 
    bf54:	4c 89 c0             	mov    %r8,%rax
    bf57:	49 89 cd             	mov    %rcx,%r13
    bf5a:	48 99                	cqto
    bf5c:	49 83 fc ff          	cmp    $0xffffffffffffffff,%r12
    bf60:	0f 85 0a 00 00 00    	jne    bf70 <py2_div_mod+0xb8>
    bf66:	ba 00 00 00 00       	mov    $0x0,%edx
    bf6b:	e9 03 00 00 00       	jmp    bf73 <py2_div_mod+0xbb>
    bf70:	49 f7 fc             	idiv   %r12
    bf73:	48 89 54 24 18       	mov    %rdx,0x18(%rsp)
    bf78:	48 8d 3d be 03 04 00 	lea    0x403be(%rip),%rdi        # 4c33d <.Ldata5>
    bf7f:	be 04 00 00 00       	mov    $0x4,%esi
    bf84:	ba 02 00 00 00       	mov    $0x2,%edx
    bf89:	4c 8d 05 50 35 00 00 	lea    0x3550(%rip),%r8        # f4e0 <pt_seq_new>
    bf90:	41 ff d0             	call   *%r8
    bf93:	49 89 c6             	mov    %rax,%r14
    bf96:	48 33 db             	xor    %rbx,%rbx
    bf99:	48 8b 54 24 10       	mov    0x10(%rsp),%rdx
    bf9e:	48 85 d2             	test   %rdx,%rdx
    bfa1:	40 0f 95 c6          	setne  %sil
    bfa5:	4c 89 e8             	mov    %r13,%rax
    bfa8:	48 33 d0             	xor    %rax,%rdx
    bfab:	48 c1 ea 3f          	shr    $0x3f,%rdx
    bfaf:	23 f2                	and    %edx,%esi
    bfb1:	48 0f b6 f6          	movzbq %sil,%rsi
    bfb5:	4c 89 f8             	mov    %r15,%rax
    bfb8:	48 89 c7             	mov    %rax,%rdi
    bfbb:	48 2b fe             	sub    %rsi,%rdi
    bfbe:	4c 8d 05 eb 35 00 00 	lea    0x35eb(%rip),%r8        # f5b0 <pt_seq_push>
    bfc5:	48 89 fe             	mov    %rdi,%rsi
    bfc8:	4c 89 f7             	mov    %r14,%rdi
    bfcb:	41 ff d0             	call   *%r8
    bfce:	48 8b 54 24 18       	mov    0x18(%rsp),%rdx
    bfd3:	48 85 d2             	test   %rdx,%rdx
    bfd6:	41 0f 95 c0          	setne  %r8b
    bfda:	48 89 d7             	mov    %rdx,%rdi
    bfdd:	49 33 fc             	xor    %r12,%rdi
    bfe0:	48 c1 ef 3f          	shr    $0x3f,%rdi
    bfe4:	44 84 c7             	test   %r8b,%dil
    bfe7:	49 0f 45 dc          	cmovne %r12,%rbx
    bfeb:	48 8d 34 1a          	lea    (%rdx,%rbx,1),%rsi
    bfef:	4c 8d 0d ba 35 00 00 	lea    0x35ba(%rip),%r9        # f5b0 <pt_seq_push>
    bff6:	4c 89 f7             	mov    %r14,%rdi
    bff9:	41 ff d1             	call   *%r9
    bffc:	4c 89 f0             	mov    %r14,%rax
    bfff:	e9 0c 00 00 00       	jmp    c010 <py2_div_mod+0x158>
    c004:	48 89 c1             	mov    %rax,%rcx
    c007:	49 89 c6             	mov    %rax,%r14
    c00a:	48 33 c1             	xor    %rcx,%rax
    c00d:	49 89 c6             	mov    %rax,%r14
    c010:	48 8b 5c 24 20       	mov    0x20(%rsp),%rbx
    c015:	4c 8b 64 24 28       	mov    0x28(%rsp),%r12
    c01a:	4c 8b 6c 24 30       	mov    0x30(%rsp),%r13
    c01f:	4c 8b 74 24 38       	mov    0x38(%rsp),%r14
    c024:	4c 8b 7c 24 40       	mov    0x40(%rsp),%r15
    c029:	48 83 c4 50          	add    $0x50,%rsp
    c02d:	48 89 ec             	mov    %rbp,%rsp
    c030:	5d                   	pop    %rbp
    c031:	c3                   	ret
    c032:	41 bb 02 00 00 00    	mov    $0x2,%r11d
    c038:	48 8d 35 30 04 04 00 	lea    0x40430(%rip),%rsi        # 4c46f <.Ldata8>
    c03f:	ba 22 00 00 00       	mov    $0x22,%edx
    c044:	48 8d 0d 65 05 04 00 	lea    0x40565(%rip),%rcx        # 4c5b0 <.Ldatac>
    c04b:	41 b8 4d 00 00 00    	mov    $0x4d,%r8d
    c051:	4c 8d 0d a5 05 04 00 	lea    0x405a5(%rip),%r9        # 4c5fd <.Ldatad>
    c058:	b8 3a 00 00 00       	mov    $0x3a,%eax
    c05d:	49 0f b6 fb          	movzbq %r11b,%rdi
    c061:	48 89 04 24          	mov    %rax,(%rsp)
    c065:	48 8d 05 d4 5f 00 00 	lea    0x5fd4(%rip),%rax        # 12040 <pt_raise_msg>
    c06c:	ff d0                	call   *%rax
    c06e:	e9 91 ff ff ff       	jmp    c004 <py2_div_mod+0x14c>
    c073:	48 8d 3d 97 04 04 00 	lea    0x40497(%rip),%rdi        # 4c511 <.Ldatab>
    c07a:	be 9f 00 00 00       	mov    $0x9f,%esi
    c07f:	48 8d 05 fa 57 00 00 	lea    0x57fa(%rip),%rax        # 11880 <pt_fail>
    c086:	ff d0                	call   *%rax
    c088:	0f 0b                	ud2
    c08a:	b8 02 00 00 00       	mov    $0x2,%eax
    c08f:	48 8d 35 d9 03 04 00 	lea    0x403d9(%rip),%rsi        # 4c46f <.Ldata8>
    c096:	ba 22 00 00 00       	mov    $0x22,%edx
    c09b:	48 8d 0d ef 03 04 00 	lea    0x403ef(%rip),%rcx        # 4c491 <.Ldata9>
    c0a2:	41 b8 4d 00 00 00    	mov    $0x4d,%r8d
    c0a8:	4c 8d 0d 2f 04 04 00 	lea    0x4042f(%rip),%r9        # 4c4de <.Ldataa>
    c0af:	41 ba 33 00 00 00    	mov    $0x33,%r10d
    c0b5:	48 0f b6 f8          	movzbq %al,%rdi
    c0b9:	4c 89 14 24          	mov    %r10,(%rsp)
    c0bd:	48 8d 05 7c 5f 00 00 	lea    0x5f7c(%rip),%rax        # 12040 <pt_raise_msg>
    c0c4:	ff d0                	call   *%rax
    c0c6:	e9 39 ff ff ff       	jmp    c004 <py2_div_mod+0x14c>
    c0cb:	0f 0b                	ud2
	...
    c0d5:	00 00                	add    %al,(%rax)
    c0d7:	80 01 00             	addb   $0x0,(%rcx)
    c0da:	00 00                	add    %al,(%rax)
    c0dc:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:

/workspace/scratch/2ce9dd46a989/card814-base/tuples:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000bad0 <pythrust_main>:
    bad0:	55                   	push   %rbp
    bad1:	48 89 e5             	mov    %rsp,%rbp
    bad4:	48 83 ec 30          	sub    $0x30,%rsp
    bad8:	48 89 1c 24          	mov    %rbx,(%rsp)
    badc:	4c 89 64 24 08       	mov    %r12,0x8(%rsp)
    bae1:	4c 89 6c 24 10       	mov    %r13,0x10(%rsp)
    bae6:	4c 89 74 24 18       	mov    %r14,0x18(%rsp)
    baeb:	4c 89 7c 24 20       	mov    %r15,0x20(%rsp)
    baf0:	4d 33 ed             	xor    %r13,%r13
    baf3:	4d 89 ee             	mov    %r13,%r14
    baf6:	4d 89 ef             	mov    %r13,%r15
    baf9:	49 81 fd 40 42 0f 00 	cmp    $0xf4240,%r13
    bb00:	0f 8c 08 00 00 00    	jl     bb0e <pythrust_main+0x3e>
    bb06:	4c 89 ff             	mov    %r15,%rdi
    bb09:	e9 89 00 00 00       	jmp    bb97 <pythrust_main+0xc7>
    bb0e:	48 89 fb             	mov    %rdi,%rbx
    bb11:	4c 8b 63 08          	mov    0x8(%rbx),%r12
    bb15:	49 81 fc e8 03 00 00 	cmp    $0x3e8,%r12
    bb1c:	0f 8d 26 01 00 00    	jge    bc48 <pythrust_main+0x178>
    bb22:	49 8d 44 24 01       	lea    0x1(%r12),%rax
    bb27:	48 89 43 08          	mov    %rax,0x8(%rbx)
    bb2b:	4c 89 ee             	mov    %r13,%rsi
    bb2e:	48 89 df             	mov    %rbx,%rdi
    bb31:	e8 71 02 00 00       	call   bda7 <pt_scalar_pair_1>
    bb36:	4c 89 63 08          	mov    %r12,0x8(%rbx)
    bb3a:	48 0f b6 0b          	movzbq (%rbx),%rcx
    bb3e:	84 c9                	test   %cl,%cl
    bb40:	0f 85 f7 00 00 00    	jne    bc3d <pythrust_main+0x16d>
    bb46:	4c 89 ff             	mov    %r15,%rdi
    bb49:	48 03 f8             	add    %rax,%rdi
    bb4c:	0f 90 c0             	seto   %al
    bb4f:	84 c0                	test   %al,%al
    bb51:	0f 85 ce 00 00 00    	jne    bc25 <pythrust_main+0x155>
    bb57:	4c 89 f0             	mov    %r14,%rax
    bb5a:	48 03 c2             	add    %rdx,%rax
    bb5d:	49 89 c6             	mov    %rax,%r14
    bb60:	0f 90 c0             	seto   %al
    bb63:	84 c0                	test   %al,%al
    bb65:	0f 85 a2 00 00 00    	jne    bc0d <pythrust_main+0x13d>
    bb6b:	4c 89 e8             	mov    %r13,%rax
    bb6e:	48 83 c0 01          	add    $0x1,%rax
    bb72:	0f 90 c0             	seto   %al
    bb75:	84 c0                	test   %al,%al
    bb77:	0f 85 1a 00 00 00    	jne    bb97 <pythrust_main+0xc7>
    bb7d:	49 83 c5 01          	add    $0x1,%r13
    bb81:	0f 90 c0             	seto   %al
    bb84:	84 c0                	test   %al,%al
    bb86:	0f 85 6a 00 00 00    	jne    bbf6 <pythrust_main+0x126>
    bb8c:	49 89 ff             	mov    %rdi,%r15
    bb8f:	48 89 df             	mov    %rbx,%rdi
    bb92:	e9 62 ff ff ff       	jmp    baf9 <pythrust_main+0x29>
    bb97:	48 8d 15 92 5d 00 00 	lea    0x5d92(%rip),%rdx        # 11930 <pt_write_int>
    bb9e:	ff d2                	call   *%rdx
    bba0:	be 20 00 00 00       	mov    $0x20,%esi
    bba5:	48 0f b6 fe          	movzbq %sil,%rdi
    bba9:	48 8d 35 50 5d 00 00 	lea    0x5d50(%rip),%rsi        # 11900 <pt_write_byte>
    bbb0:	ff d6                	call   *%rsi
    bbb2:	48 8d 35 77 5d 00 00 	lea    0x5d77(%rip),%rsi        # 11930 <pt_write_int>
    bbb9:	4c 89 f7             	mov    %r14,%rdi
    bbbc:	ff d6                	call   *%rsi
    bbbe:	be 0a 00 00 00       	mov    $0xa,%esi
    bbc3:	48 0f b6 fe          	movzbq %sil,%rdi
    bbc7:	48 8d 35 32 5d 00 00 	lea    0x5d32(%rip),%rsi        # 11900 <pt_write_byte>
    bbce:	ff d6                	call   *%rsi
    bbd0:	48 8b 1c 24          	mov    (%rsp),%rbx
    bbd4:	4c 8b 64 24 08       	mov    0x8(%rsp),%r12
    bbd9:	4c 8b 6c 24 10       	mov    0x10(%rsp),%r13
    bbde:	4c 8b 74 24 18       	mov    0x18(%rsp),%r14
    bbe3:	4c 8b 7c 24 20       	mov    0x20(%rsp),%r15
    bbe8:	48 83 c4 30          	add    $0x30,%rsp
    bbec:	48 89 ec             	mov    %rbp,%rsp
    bbef:	5d                   	pop    %rbp
    bbf0:	c3                   	ret
    bbf1:	e9 47 00 00 00       	jmp    bc3d <pythrust_main+0x16d>
    bbf6:	48 8d 3d b0 04 04 00 	lea    0x404b0(%rip),%rdi        # 4c0ad <.Ldata1>
    bbfd:	be a3 00 00 00       	mov    $0xa3,%esi
    bc02:	48 8d 05 77 5c 00 00 	lea    0x5c77(%rip),%rax        # 11880 <pt_fail>
    bc09:	ff d0                	call   *%rax
    bc0b:	0f 0b                	ud2
    bc0d:	48 8d 3d d3 05 04 00 	lea    0x405d3(%rip),%rdi        # 4c1e7 <.Ldata3>
    bc14:	be 97 00 00 00       	mov    $0x97,%esi
    bc19:	4c 8d 05 60 5c 00 00 	lea    0x5c60(%rip),%r8        # 11880 <pt_fail>
    bc20:	41 ff d0             	call   *%r8
    bc23:	0f 0b                	ud2
    bc25:	48 8d 3d 24 05 04 00 	lea    0x40524(%rip),%rdi        # 4c150 <.Ldata2>
    bc2c:	be 97 00 00 00       	mov    $0x97,%esi
    bc31:	4c 8d 15 48 5c 00 00 	lea    0x5c48(%rip),%r10        # 11880 <pt_fail>
    bc38:	41 ff d2             	call   *%r10
    bc3b:	0f 0b                	ud2
    bc3d:	48 8d 05 ec 64 00 00 	lea    0x64ec(%rip),%rax        # 12130 <pt_uncaught>
    bc44:	ff d0                	call   *%rax
    bc46:	0f 0b                	ud2
    bc48:	48 8d 3d b1 03 04 00 	lea    0x403b1(%rip),%rdi        # 4c000 <.Ldata0>
    bc4f:	be ad 00 00 00       	mov    $0xad,%esi
    bc54:	48 8d 05 25 5c 00 00 	lea    0x5c25(%rip),%rax        # 11880 <pt_fail>
    bc5b:	ff d0                	call   *%rax
    bc5d:	0f 0b                	ud2

Disassembly of section .fini:
