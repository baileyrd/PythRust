
/workspace/scratch/2ce9dd46a989/card814-base/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000c360 <py1_quicksort>:
    c360:	55                   	push   %rbp
    c361:	48 89 e5             	mov    %rsp,%rbp
    c364:	48 81 ec b0 00 00 00 	sub    $0xb0,%rsp
    c36b:	48 89 9c 24 80 00 00 	mov    %rbx,0x80(%rsp)
    c372:	00 
    c373:	4c 89 a4 24 88 00 00 	mov    %r12,0x88(%rsp)
    c37a:	00 
    c37b:	4c 89 ac 24 90 00 00 	mov    %r13,0x90(%rsp)
    c382:	00 
    c383:	4c 89 b4 24 98 00 00 	mov    %r14,0x98(%rsp)
    c38a:	00 
    c38b:	4c 89 bc 24 a0 00 00 	mov    %r15,0xa0(%rsp)
    c392:	00 
    c393:	49 89 ff             	mov    %rdi,%r15
    c396:	48 89 74 24 38       	mov    %rsi,0x38(%rsp)
    c39b:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c3a0:	48 89 4c 24 58       	mov    %rcx,0x58(%rsp)
    c3a5:	48 8d 4c 24 10       	lea    0x10(%rsp),%rcx
    c3aa:	48 89 4c 24 78       	mov    %rcx,0x78(%rsp)
    c3af:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c3b4:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c3b9:	48 3b d1             	cmp    %rcx,%rdx
    c3bc:	0f 8d f0 03 00 00    	jge    c7b2 <py1_quicksort+0x452>
    c3c2:	48 89 d0             	mov    %rdx,%rax
    c3c5:	48 03 c1             	add    %rcx,%rax
    c3c8:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c3cd:	48 89 4c 24 58       	mov    %rcx,0x58(%rsp)
    c3d2:	0f 90 c1             	seto   %cl
    c3d5:	84 c9                	test   %cl,%cl
    c3d7:	0f 85 5b 08 00 00    	jne    cc38 <py1_quicksort+0x8d8>
    c3dd:	48 89 c1             	mov    %rax,%rcx
    c3e0:	48 c1 e9 3f          	shr    $0x3f,%rcx
    c3e4:	48 03 c8             	add    %rax,%rcx
    c3e7:	48 89 ce             	mov    %rcx,%rsi
    c3ea:	48 d1 fe             	sar    $1,%rsi
    c3ed:	48 83 e1 fe          	and    $0xfffffffffffffffe,%rcx
    c3f1:	48 3b c1             	cmp    %rcx,%rax
    c3f4:	0f 95 c2             	setne  %dl
    c3f7:	48 2b c1             	sub    %rcx,%rax
    c3fa:	48 83 f0 02          	xor    $0x2,%rax
    c3fe:	48 c1 e8 3f          	shr    $0x3f,%rax
    c402:	23 d0                	and    %eax,%edx
    c404:	48 0f b6 c2          	movzbq %dl,%rax
    c408:	48 2b f0             	sub    %rax,%rsi
    c40b:	48 8d 05 ee 3c 00 00 	lea    0x3cee(%rip),%rax        # 10100 <pt_seq_get>
    c412:	48 8b 54 24 78       	mov    0x78(%rsp),%rdx
    c417:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c41c:	ff d0                	call   *%rax
    c41e:	48 89 44 24 70       	mov    %rax,0x70(%rsp)
    c423:	48 0f b6 44 24 10    	movzbq 0x10(%rsp),%rax
    c429:	3c 01                	cmp    $0x1,%al
    c42b:	0f 84 c5 07 00 00    	je     cbf6 <py1_quicksort+0x896>
    c431:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c437:	48 8d 5c 24 28       	lea    0x28(%rsp),%rbx
    c43c:	48 89 5c 24 68       	mov    %rbx,0x68(%rsp)
    c441:	4c 8d 64 24 30       	lea    0x30(%rsp),%r12
    c446:	4c 89 64 24 60       	mov    %r12,0x60(%rsp)
    c44b:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c450:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c455:	4d 89 f5             	mov    %r14,%r13
    c458:	45 84 f6             	test   %r14b,%r14b
    c45b:	0f 84 7d 07 00 00    	je     cbde <py1_quicksort+0x87e>
    c461:	45 84 ed             	test   %r13b,%r13b
    c464:	0f 84 5c 07 00 00    	je     cbc6 <py1_quicksort+0x866>
    c46a:	48 8b 44 24 40       	mov    0x40(%rsp),%rax
    c46f:	48 3b c1             	cmp    %rcx,%rax
    c472:	0f 8e 26 01 00 00    	jle    c59e <py1_quicksort+0x23e>
    c478:	45 84 ed             	test   %r13b,%r13b
    c47b:	0f 84 34 04 00 00    	je     c8b5 <py1_quicksort+0x555>
    c481:	49 89 c8             	mov    %rcx,%r8
    c484:	4c 2b c2             	sub    %rdx,%r8
    c487:	48 89 4c 24 48       	mov    %rcx,0x48(%rsp)
    c48c:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
    c491:	41 0f 90 c2          	seto   %r10b
    c495:	45 84 d2             	test   %r10b,%r10b
    c498:	0f 85 00 04 00 00    	jne    c89e <py1_quicksort+0x53e>
    c49e:	45 84 f6             	test   %r14b,%r14b
    c4a1:	0f 84 e0 03 00 00    	je     c887 <py1_quicksort+0x527>
    c4a7:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c4ac:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c4b1:	48 89 c8             	mov    %rcx,%rax
    c4b4:	48 2b c2             	sub    %rdx,%rax
    c4b7:	0f 90 c1             	seto   %cl
    c4ba:	84 c9                	test   %cl,%cl
    c4bc:	0f 85 ae 03 00 00    	jne    c870 <py1_quicksort+0x510>
    c4c2:	4c 3b c0             	cmp    %rax,%r8
    c4c5:	0f 8c 72 00 00 00    	jl     c53d <py1_quicksort+0x1dd>
    c4cb:	45 84 f6             	test   %r14b,%r14b
    c4ce:	0f 84 40 03 00 00    	je     c814 <py1_quicksort+0x4b4>
    c4d4:	4d 89 fc             	mov    %r15,%r12
    c4d7:	49 8b 5c 24 08       	mov    0x8(%r12),%rbx
    c4dc:	48 81 fb e8 03 00 00 	cmp    $0x3e8,%rbx
    c4e3:	0f 8d 14 03 00 00    	jge    c7fd <py1_quicksort+0x49d>
    c4e9:	48 8d 43 01          	lea    0x1(%rbx),%rax
    c4ed:	49 89 44 24 08       	mov    %rax,0x8(%r12)
    c4f2:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c4f7:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c4fc:	48 8b 74 24 38       	mov    0x38(%rsp),%rsi
    c501:	4c 89 e7             	mov    %r12,%rdi
    c504:	e8 57 fe ff ff       	call   c360 <py1_quicksort>
    c509:	49 89 5c 24 08       	mov    %rbx,0x8(%r12)
    c50e:	49 0f b6 04 24       	movzbq (%r12),%rax
    c513:	84 c0                	test   %al,%al
    c515:	0f 85 97 02 00 00    	jne    c7b2 <py1_quicksort+0x452>
    c51b:	45 84 ed             	test   %r13b,%r13b
    c51e:	0f 84 c2 02 00 00    	je     c7e6 <py1_quicksort+0x486>
    c524:	48 8b 44 24 50       	mov    0x50(%rsp),%rax
    c529:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
    c52e:	48 8b 44 24 48       	mov    0x48(%rsp),%rax
    c533:	48 89 44 24 58       	mov    %rax,0x58(%rsp)
    c538:	e9 59 00 00 00       	jmp    c596 <py1_quicksort+0x236>
    c53d:	4d 89 fc             	mov    %r15,%r12
    c540:	45 84 ed             	test   %r13b,%r13b
    c543:	0f 84 10 03 00 00    	je     c859 <py1_quicksort+0x4f9>
    c549:	49 8b 5c 24 08       	mov    0x8(%r12),%rbx
    c54e:	48 81 fb e8 03 00 00 	cmp    $0x3e8,%rbx
    c555:	0f 8d e7 02 00 00    	jge    c842 <py1_quicksort+0x4e2>
    c55b:	48 8d 43 01          	lea    0x1(%rbx),%rax
    c55f:	49 89 44 24 08       	mov    %rax,0x8(%r12)
    c564:	48 8b 4c 24 48       	mov    0x48(%rsp),%rcx
    c569:	48 8b 54 24 50       	mov    0x50(%rsp),%rdx
    c56e:	48 8b 74 24 38       	mov    0x38(%rsp),%rsi
    c573:	4c 89 e7             	mov    %r12,%rdi
    c576:	e8 e5 fd ff ff       	call   c360 <py1_quicksort>
    c57b:	49 89 5c 24 08       	mov    %rbx,0x8(%r12)
    c580:	49 0f b6 04 24       	movzbq (%r12),%rax
    c585:	84 c0                	test   %al,%al
    c587:	0f 85 25 02 00 00    	jne    c7b2 <py1_quicksort+0x452>
    c58d:	45 84 f6             	test   %r14b,%r14b
    c590:	0f 84 95 02 00 00    	je     c82b <py1_quicksort+0x4cb>
    c596:	4d 89 e7             	mov    %r12,%r15
    c599:	e9 11 fe ff ff       	jmp    c3af <py1_quicksort+0x4f>
    c59e:	4d 89 fc             	mov    %r15,%r12
    c5a1:	48 89 4c 24 48       	mov    %rcx,0x48(%rsp)
    c5a6:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
    c5ab:	48 8d 5c 24 18       	lea    0x18(%rsp),%rbx
    c5b0:	45 84 f6             	test   %r14b,%r14b
    c5b3:	0f 84 f6 05 00 00    	je     cbaf <py1_quicksort+0x84f>
    c5b9:	4c 8d 05 40 3b 00 00 	lea    0x3b40(%rip),%r8        # 10100 <pt_seq_get>
    c5c0:	48 89 da             	mov    %rbx,%rdx
    c5c3:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c5c8:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c5cd:	41 ff d0             	call   *%r8
    c5d0:	48 0f b6 74 24 18    	movzbq 0x18(%rsp),%rsi
    c5d6:	40 80 fe 01          	cmp    $0x1,%sil
    c5da:	0f 84 8e 05 00 00    	je     cb6e <py1_quicksort+0x80e>
    c5e0:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
    c5e5:	48 3b c1             	cmp    %rcx,%rax
    c5e8:	0f 8c 8d 01 00 00    	jl     c77b <py1_quicksort+0x41b>
    c5ee:	48 8d 5c 24 20       	lea    0x20(%rsp),%rbx
    c5f3:	45 84 ed             	test   %r13b,%r13b
    c5f6:	0f 84 2d 05 00 00    	je     cb29 <py1_quicksort+0x7c9>
    c5fc:	48 8d 05 fd 3a 00 00 	lea    0x3afd(%rip),%rax        # 10100 <pt_seq_get>
    c603:	4c 8b 7c 24 48       	mov    0x48(%rsp),%r15
    c608:	48 89 da             	mov    %rbx,%rdx
    c60b:	4c 89 fe             	mov    %r15,%rsi
    c60e:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c613:	ff d0                	call   *%rax
    c615:	48 0f b6 4c 24 20    	movzbq 0x20(%rsp),%rcx
    c61b:	40 80 f9 01          	rex cmp $0x1,%cl
    c61f:	0f 84 c3 04 00 00    	je     cae8 <py1_quicksort+0x788>
    c625:	48 8b 74 24 70       	mov    0x70(%rsp),%rsi
    c62a:	48 3b c6             	cmp    %rsi,%rax
    c62d:	0f 8f 20 01 00 00    	jg     c753 <py1_quicksort+0x3f3>
    c633:	45 84 f6             	test   %r14b,%r14b
    c636:	0f 84 67 04 00 00    	je     caa3 <py1_quicksort+0x743>
    c63c:	45 84 ed             	test   %r13b,%r13b
    c63f:	0f 84 47 04 00 00    	je     ca8c <py1_quicksort+0x72c>
    c645:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c64a:	49 3b d7             	cmp    %r15,%rdx
    c64d:	0f 8f f0 00 00 00    	jg     c743 <py1_quicksort+0x3e3>
    c653:	45 84 f6             	test   %r14b,%r14b
    c656:	0f 84 19 04 00 00    	je     ca75 <py1_quicksort+0x715>
    c65c:	48 8d 05 9d 3a 00 00 	lea    0x3a9d(%rip),%rax        # 10100 <pt_seq_get>
    c663:	48 8b 54 24 68       	mov    0x68(%rsp),%rdx
    c668:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c66d:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c672:	ff d0                	call   *%rax
    c674:	48 89 c3             	mov    %rax,%rbx
    c677:	48 0f b6 44 24 28    	movzbq 0x28(%rsp),%rax
    c67d:	3c 01                	cmp    $0x1,%al
    c67f:	0f 84 af 03 00 00    	je     ca34 <py1_quicksort+0x6d4>
    c685:	45 84 ed             	test   %r13b,%r13b
    c688:	0f 84 8f 03 00 00    	je     ca1d <py1_quicksort+0x6bd>
    c68e:	48 8d 05 6b 3a 00 00 	lea    0x3a6b(%rip),%rax        # 10100 <pt_seq_get>
    c695:	48 8b 54 24 60       	mov    0x60(%rsp),%rdx
    c69a:	4c 89 fe             	mov    %r15,%rsi
    c69d:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c6a2:	ff d0                	call   *%rax
    c6a4:	48 0f b6 4c 24 30    	movzbq 0x30(%rsp),%rcx
    c6aa:	40 80 f9 01          	rex cmp $0x1,%cl
    c6ae:	0f 84 28 03 00 00    	je     c9dc <py1_quicksort+0x67c>
    c6b4:	45 84 f6             	test   %r14b,%r14b
    c6b7:	0f 84 08 03 00 00    	je     c9c5 <py1_quicksort+0x665>
    c6bd:	48 8d 0d 1c 37 00 00 	lea    0x371c(%rip),%rcx        # fde0 <pt_list_set>
    c6c4:	48 89 c2             	mov    %rax,%rdx
    c6c7:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c6cc:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c6d1:	ff d1                	call   *%rcx
    c6d3:	3c 01                	cmp    $0x1,%al
    c6d5:	0f 84 a9 02 00 00    	je     c984 <py1_quicksort+0x624>
    c6db:	45 84 ed             	test   %r13b,%r13b
    c6de:	0f 84 89 02 00 00    	je     c96d <py1_quicksort+0x60d>
    c6e4:	48 8d 05 f5 36 00 00 	lea    0x36f5(%rip),%rax        # fde0 <pt_list_set>
    c6eb:	48 89 da             	mov    %rbx,%rdx
    c6ee:	4c 89 fe             	mov    %r15,%rsi
    c6f1:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c6f6:	ff d0                	call   *%rax
    c6f8:	3c 01                	cmp    $0x1,%al
    c6fa:	0f 84 2a 02 00 00    	je     c92a <py1_quicksort+0x5ca>
    c700:	45 84 f6             	test   %r14b,%r14b
    c703:	0f 84 09 02 00 00    	je     c912 <py1_quicksort+0x5b2>
    c709:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c70e:	48 83 c2 01          	add    $0x1,%rdx
    c712:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c717:	0f 90 c0             	seto   %al
    c71a:	84 c0                	test   %al,%al
    c71c:	0f 85 d8 01 00 00    	jne    c8fa <py1_quicksort+0x59a>
    c722:	45 84 ed             	test   %r13b,%r13b
    c725:	0f 84 b8 01 00 00    	je     c8e3 <py1_quicksort+0x583>
    c72b:	49 83 ef 01          	sub    $0x1,%r15
    c72f:	0f 90 c0             	seto   %al
    c732:	84 c0                	test   %al,%al
    c734:	0f 85 92 01 00 00    	jne    c8cc <py1_quicksort+0x56c>
    c73a:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c740:	4d 89 f5             	mov    %r14,%r13
    c743:	48 8b 54 24 50       	mov    0x50(%rsp),%rdx
    c748:	4c 89 f9             	mov    %r15,%rcx
    c74b:	4d 89 e7             	mov    %r12,%r15
    c74e:	e9 05 fd ff ff       	jmp    c458 <py1_quicksort+0xf8>
    c753:	45 84 ed             	test   %r13b,%r13b
    c756:	0f 84 75 03 00 00    	je     cad1 <py1_quicksort+0x771>
    c75c:	49 83 ef 01          	sub    $0x1,%r15
    c760:	0f 90 c0             	seto   %al
    c763:	84 c0                	test   %al,%al
    c765:	0f 85 4f 03 00 00    	jne    caba <py1_quicksort+0x75a>
    c76b:	41 bd 01 00 00 00    	mov    $0x1,%r13d
    c771:	4c 89 7c 24 48       	mov    %r15,0x48(%rsp)
    c776:	e9 78 fe ff ff       	jmp    c5f3 <py1_quicksort+0x293>
    c77b:	4c 8b 7c 24 48       	mov    0x48(%rsp),%r15
    c780:	45 84 f6             	test   %r14b,%r14b
    c783:	0f 84 ce 03 00 00    	je     cb57 <py1_quicksort+0x7f7>
    c789:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c78e:	48 83 c2 01          	add    $0x1,%rdx
    c792:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c797:	0f 90 c0             	seto   %al
    c79a:	84 c0                	test   %al,%al
    c79c:	0f 85 9e 03 00 00    	jne    cb40 <py1_quicksort+0x7e0>
    c7a2:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c7a8:	4c 89 7c 24 48       	mov    %r15,0x48(%rsp)
    c7ad:	e9 fe fd ff ff       	jmp    c5b0 <py1_quicksort+0x250>
    c7b2:	48 8b 9c 24 80 00 00 	mov    0x80(%rsp),%rbx
    c7b9:	00 
    c7ba:	4c 8b a4 24 88 00 00 	mov    0x88(%rsp),%r12
    c7c1:	00 
    c7c2:	4c 8b ac 24 90 00 00 	mov    0x90(%rsp),%r13
    c7c9:	00 
    c7ca:	4c 8b b4 24 98 00 00 	mov    0x98(%rsp),%r14
    c7d1:	00 
    c7d2:	4c 8b bc 24 a0 00 00 	mov    0xa0(%rsp),%r15
    c7d9:	00 
    c7da:	48 81 c4 b0 00 00 00 	add    $0xb0,%rsp
    c7e1:	48 89 ec             	mov    %rbp,%rsp
    c7e4:	5d                   	pop    %rbp
    c7e5:	c3                   	ret
    c7e6:	48 8d 3d cd 33 04 00 	lea    0x433cd(%rip),%rdi        # 4fbba <.Ldata4f>
    c7ed:	be d5 00 00 00       	mov    $0xd5,%esi
    c7f2:	48 8d 05 c7 5d 00 00 	lea    0x5dc7(%rip),%rax        # 125c0 <pt_fail>
    c7f9:	ff d0                	call   *%rax
    c7fb:	0f 0b                	ud2
    c7fd:	48 8d 3d f0 32 04 00 	lea    0x432f0(%rip),%rdi        # 4faf4 <.Ldata4e>
    c804:	be c6 00 00 00       	mov    $0xc6,%esi
    c809:	48 8d 05 b0 5d 00 00 	lea    0x5db0(%rip),%rax        # 125c0 <pt_fail>
    c810:	ff d0                	call   *%rax
    c812:	0f 0b                	ud2
    c814:	48 8d 3d ed 31 04 00 	lea    0x431ed(%rip),%rdi        # 4fa08 <.Ldata4d>
    c81b:	be ec 00 00 00       	mov    $0xec,%esi
    c820:	48 8d 05 99 5d 00 00 	lea    0x5d99(%rip),%rax        # 125c0 <pt_fail>
    c827:	ff d0                	call   *%rax
    c829:	0f 0b                	ud2
    c82b:	48 8d 3d 01 31 04 00 	lea    0x43101(%rip),%rdi        # 4f933 <.Ldata4c>
    c832:	be d5 00 00 00       	mov    $0xd5,%esi
    c837:	48 8d 05 82 5d 00 00 	lea    0x5d82(%rip),%rax        # 125c0 <pt_fail>
    c83e:	ff d0                	call   *%rax
    c840:	0f 0b                	ud2
    c842:	48 8d 3d 24 30 04 00 	lea    0x43024(%rip),%rdi        # 4f86d <.Ldata4b>
    c849:	be c6 00 00 00       	mov    $0xc6,%esi
    c84e:	48 8d 05 6b 5d 00 00 	lea    0x5d6b(%rip),%rax        # 125c0 <pt_fail>
    c855:	ff d0                	call   *%rax
    c857:	0f 0b                	ud2
    c859:	48 8d 3d 1d 2f 04 00 	lea    0x42f1d(%rip),%rdi        # 4f77d <.Ldata4a>
    c860:	be f0 00 00 00       	mov    $0xf0,%esi
    c865:	48 8d 05 54 5d 00 00 	lea    0x5d54(%rip),%rax        # 125c0 <pt_fail>
    c86c:	ff d0                	call   *%rax
    c86e:	0f 0b                	ud2
    c870:	48 8d 3d 5c 2e 04 00 	lea    0x42e5c(%rip),%rdi        # 4f6d3 <.Ldata49>
    c877:	be aa 00 00 00       	mov    $0xaa,%esi
    c87c:	48 8d 05 3d 5d 00 00 	lea    0x5d3d(%rip),%rax        # 125c0 <pt_fail>
    c883:	ff d0                	call   *%rax
    c885:	0f 0b                	ud2
    c887:	48 8d 3d 5f 2d 04 00 	lea    0x42d5f(%rip),%rdi        # 4f5ed <.Ldata48>
    c88e:	be e6 00 00 00       	mov    $0xe6,%esi
    c893:	48 8d 05 26 5d 00 00 	lea    0x5d26(%rip),%rax        # 125c0 <pt_fail>
    c89a:	ff d0                	call   *%rax
    c89c:	0f 0b                	ud2
    c89e:	48 8d 3d a7 2c 04 00 	lea    0x42ca7(%rip),%rdi        # 4f54c <.Ldata47>
    c8a5:	be a1 00 00 00       	mov    $0xa1,%esi
    c8aa:	48 8d 05 0f 5d 00 00 	lea    0x5d0f(%rip),%rax        # 125c0 <pt_fail>
    c8b1:	ff d0                	call   *%rax
    c8b3:	0f 0b                	ud2
    c8b5:	48 8d 3d b8 2b 04 00 	lea    0x42bb8(%rip),%rdi        # 4f474 <.Ldata46>
    c8bc:	be d8 00 00 00       	mov    $0xd8,%esi
    c8c1:	48 8d 05 f8 5c 00 00 	lea    0x5cf8(%rip),%rax        # 125c0 <pt_fail>
    c8c8:	ff d0                	call   *%rax
    c8ca:	0f 0b                	ud2
    c8cc:	48 8d 3d 00 2b 04 00 	lea    0x42b00(%rip),%rdi        # 4f3d3 <.Ldata45>
    c8d3:	be a1 00 00 00       	mov    $0xa1,%esi
    c8d8:	48 8d 05 e1 5c 00 00 	lea    0x5ce1(%rip),%rax        # 125c0 <pt_fail>
    c8df:	ff d0                	call   *%rax
    c8e1:	0f 0b                	ud2
    c8e3:	48 8d 3d 11 2a 04 00 	lea    0x42a11(%rip),%rdi        # 4f2fb <.Ldata44>
    c8ea:	be d8 00 00 00       	mov    $0xd8,%esi
    c8ef:	48 8d 15 ca 5c 00 00 	lea    0x5cca(%rip),%rdx        # 125c0 <pt_fail>
    c8f6:	ff d2                	call   *%rdx
    c8f8:	0f 0b                	ud2
    c8fa:	48 8d 3d 59 29 04 00 	lea    0x42959(%rip),%rdi        # 4f25a <.Ldata43>
    c901:	be a1 00 00 00       	mov    $0xa1,%esi
    c906:	4c 8d 05 b3 5c 00 00 	lea    0x5cb3(%rip),%r8        # 125c0 <pt_fail>
    c90d:	41 ff d0             	call   *%r8
    c910:	0f 0b                	ud2
    c912:	48 8d 3d 69 28 04 00 	lea    0x42869(%rip),%rdi        # 4f182 <.Ldata42>
    c919:	be d8 00 00 00       	mov    $0xd8,%esi
    c91e:	4c 8d 05 9b 5c 00 00 	lea    0x5c9b(%rip),%r8        # 125c0 <pt_fail>
    c925:	41 ff d0             	call   *%r8
    c928:	0f 0b                	ud2
    c92a:	41 ba 01 00 00 00    	mov    $0x1,%r10d
    c930:	48 8d 35 5c 25 04 00 	lea    0x4255c(%rip),%rsi        # 4ee93 <.Ldata3b>
    c937:	ba 22 00 00 00       	mov    $0x22,%edx
    c93c:	48 8d 0d bb 27 04 00 	lea    0x427bb(%rip),%rcx        # 4f0fe <.Ldata40>
    c943:	41 b8 4f 00 00 00    	mov    $0x4f,%r8d
    c949:	4c 8d 0d fd 27 04 00 	lea    0x427fd(%rip),%r9        # 4f14d <.Ldata41>
    c950:	41 bb 35 00 00 00    	mov    $0x35,%r11d
    c956:	49 0f b6 fa          	movzbq %r10b,%rdi
    c95a:	4c 89 1c 24          	mov    %r11,(%rsp)
    c95e:	4c 8d 15 5b 64 00 00 	lea    0x645b(%rip),%r10        # 12dc0 <pt_raise_msg>
    c965:	41 ff d2             	call   *%r10
    c968:	e9 45 fe ff ff       	jmp    c7b2 <py1_quicksort+0x452>
    c96d:	48 8d 3d ac 26 04 00 	lea    0x426ac(%rip),%rdi        # 4f020 <.Ldata3f>
    c974:	be de 00 00 00       	mov    $0xde,%esi
    c979:	48 8d 05 40 5c 00 00 	lea    0x5c40(%rip),%rax        # 125c0 <pt_fail>
    c980:	ff d0                	call   *%rax
    c982:	0f 0b                	ud2
    c984:	b8 01 00 00 00       	mov    $0x1,%eax
    c989:	48 8d 35 03 25 04 00 	lea    0x42503(%rip),%rsi        # 4ee93 <.Ldata3b>
    c990:	ba 22 00 00 00       	mov    $0x22,%edx
    c995:	48 8d 0d 19 25 04 00 	lea    0x42519(%rip),%rcx        # 4eeb5 <.Ldata3c>
    c99c:	41 b8 4f 00 00 00    	mov    $0x4f,%r8d
    c9a2:	4c 8d 0d 5b 25 04 00 	lea    0x4255b(%rip),%r9        # 4ef04 <.Ldata3d>
    c9a9:	41 ba 39 00 00 00    	mov    $0x39,%r10d
    c9af:	48 0f b6 f8          	movzbq %al,%rdi
    c9b3:	4c 89 14 24          	mov    %r10,(%rsp)
    c9b7:	48 8d 05 02 64 00 00 	lea    0x6402(%rip),%rax        # 12dc0 <pt_raise_msg>
    c9be:	ff d0                	call   *%rax
    c9c0:	e9 ed fd ff ff       	jmp    c7b2 <py1_quicksort+0x452>
    c9c5:	48 8d 3d e5 23 04 00 	lea    0x423e5(%rip),%rdi        # 4edb1 <.Ldata3a>
    c9cc:	be e2 00 00 00       	mov    $0xe2,%esi
    c9d1:	48 8d 05 e8 5b 00 00 	lea    0x5be8(%rip),%rax        # 125c0 <pt_fail>
    c9d8:	ff d0                	call   *%rax
    c9da:	0f 0b                	ud2
    c9dc:	b8 01 00 00 00       	mov    $0x1,%eax
    c9e1:	48 8d 35 c6 0b 04 00 	lea    0x40bc6(%rip),%rsi        # 4d5ae <.Ldatac>
    c9e8:	ba 17 00 00 00       	mov    $0x17,%edx
    c9ed:	48 8d 0d 2d 23 04 00 	lea    0x4232d(%rip),%rcx        # 4ed21 <.Ldata38>
    c9f4:	41 b8 4f 00 00 00    	mov    $0x4f,%r8d
    c9fa:	4c 8d 0d 6f 23 04 00 	lea    0x4236f(%rip),%r9        # 4ed70 <.Ldata39>
    ca01:	41 ba 41 00 00 00    	mov    $0x41,%r10d
    ca07:	48 0f b6 f8          	movzbq %al,%rdi
    ca0b:	4c 89 14 24          	mov    %r10,(%rsp)
    ca0f:	48 8d 05 aa 63 00 00 	lea    0x63aa(%rip),%rax        # 12dc0 <pt_raise_msg>
    ca16:	ff d0                	call   *%rax
    ca18:	e9 95 fd ff ff       	jmp    c7b2 <py1_quicksort+0x452>
    ca1d:	48 8d 3d 13 22 04 00 	lea    0x42213(%rip),%rdi        # 4ec37 <.Ldata37>
    ca24:	be ea 00 00 00       	mov    $0xea,%esi
    ca29:	48 8d 05 90 5b 00 00 	lea    0x5b90(%rip),%rax        # 125c0 <pt_fail>
    ca30:	ff d0                	call   *%rax
    ca32:	0f 0b                	ud2
    ca34:	b8 01 00 00 00       	mov    $0x1,%eax
    ca39:	48 8d 35 6e 0b 04 00 	lea    0x40b6e(%rip),%rsi        # 4d5ae <.Ldatac>
    ca40:	ba 17 00 00 00       	mov    $0x17,%edx
    ca45:	48 8d 0d 63 21 04 00 	lea    0x42163(%rip),%rcx        # 4ebaf <.Ldata35>
    ca4c:	41 b8 4f 00 00 00    	mov    $0x4f,%r8d
    ca52:	4c 8d 0d a5 21 04 00 	lea    0x421a5(%rip),%r9        # 4ebfe <.Ldata36>
    ca59:	41 ba 39 00 00 00    	mov    $0x39,%r10d
    ca5f:	48 0f b6 f8          	movzbq %al,%rdi
    ca63:	4c 89 14 24          	mov    %r10,(%rsp)
    ca67:	48 8d 05 52 63 00 00 	lea    0x6352(%rip),%rax        # 12dc0 <pt_raise_msg>
    ca6e:	ff d0                	call   *%rax
    ca70:	e9 3d fd ff ff       	jmp    c7b2 <py1_quicksort+0x452>
    ca75:	48 8d 3d 51 20 04 00 	lea    0x42051(%rip),%rdi        # 4eacd <.Ldata34>
    ca7c:	be e2 00 00 00       	mov    $0xe2,%esi
    ca81:	48 8d 05 38 5b 00 00 	lea    0x5b38(%rip),%rax        # 125c0 <pt_fail>
    ca88:	ff d0                	call   *%rax
    ca8a:	0f 0b                	ud2
    ca8c:	48 8d 3d 5e 1f 04 00 	lea    0x41f5e(%rip),%rdi        # 4e9f1 <.Ldata33>
    ca93:	be dc 00 00 00       	mov    $0xdc,%esi
    ca98:	48 8d 05 21 5b 00 00 	lea    0x5b21(%rip),%rax        # 125c0 <pt_fail>
    ca9f:	ff d0                	call   *%rax
    caa1:	0f 0b                	ud2
    caa3:	48 8d 3d 70 1e 04 00 	lea    0x41e70(%rip),%rdi        # 4e91a <.Ldata32>
    caaa:	be d7 00 00 00       	mov    $0xd7,%esi
    caaf:	48 8d 05 0a 5b 00 00 	lea    0x5b0a(%rip),%rax        # 125c0 <pt_fail>
    cab6:	ff d0                	call   *%rax
    cab8:	0f 0b                	ud2
    caba:	48 8d 3d b8 1d 04 00 	lea    0x41db8(%rip),%rdi        # 4e879 <.Ldata31>
    cac1:	be a1 00 00 00       	mov    $0xa1,%esi
    cac6:	48 8d 05 f3 5a 00 00 	lea    0x5af3(%rip),%rax        # 125c0 <pt_fail>
    cacd:	ff d0                	call   *%rax
    cacf:	0f 0b                	ud2
    cad1:	48 8d 3d c9 1c 04 00 	lea    0x41cc9(%rip),%rdi        # 4e7a1 <.Ldata30>
    cad8:	be d8 00 00 00       	mov    $0xd8,%esi
    cadd:	48 8d 05 dc 5a 00 00 	lea    0x5adc(%rip),%rax        # 125c0 <pt_fail>
    cae4:	ff d0                	call   *%rax
    cae6:	0f 0b                	ud2
    cae8:	b8 01 00 00 00       	mov    $0x1,%eax
    caed:	48 8d 35 ba 0a 04 00 	lea    0x40aba(%rip),%rsi        # 4d5ae <.Ldatac>
    caf4:	ba 17 00 00 00       	mov    $0x17,%edx
    caf9:	48 8d 0d 20 1b 04 00 	lea    0x41b20(%rip),%rcx        # 4e620 <.Ldata2d>
    cb00:	41 b8 4f 00 00 00    	mov    $0x4f,%r8d
    cb06:	4c 8d 0d 62 1b 04 00 	lea    0x41b62(%rip),%r9        # 4e66f <.Ldata2e>
    cb0d:	41 ba 3e 00 00 00    	mov    $0x3e,%r10d
    cb13:	48 0f b6 f8          	movzbq %al,%rdi
    cb17:	4c 89 14 24          	mov    %r10,(%rsp)
    cb1b:	48 8d 05 9e 62 00 00 	lea    0x629e(%rip),%rax        # 12dc0 <pt_raise_msg>
    cb22:	ff d0                	call   *%rax
    cb24:	e9 89 fc ff ff       	jmp    c7b2 <py1_quicksort+0x452>
    cb29:	48 8d 3d 09 1a 04 00 	lea    0x41a09(%rip),%rdi        # 4e539 <.Ldata2c>
    cb30:	be e7 00 00 00       	mov    $0xe7,%esi
    cb35:	48 8d 05 84 5a 00 00 	lea    0x5a84(%rip),%rax        # 125c0 <pt_fail>
    cb3c:	ff d0                	call   *%rax
    cb3e:	0f 0b                	ud2
    cb40:	48 8d 3d 52 19 04 00 	lea    0x41952(%rip),%rdi        # 4e499 <.Ldata2b>
    cb47:	be a0 00 00 00       	mov    $0xa0,%esi
    cb4c:	48 8d 05 6d 5a 00 00 	lea    0x5a6d(%rip),%rax        # 125c0 <pt_fail>
    cb53:	ff d0                	call   *%rax
    cb55:	0f 0b                	ud2
    cb57:	48 8d 3d 64 18 04 00 	lea    0x41864(%rip),%rdi        # 4e3c2 <.Ldata2a>
    cb5e:	be d7 00 00 00       	mov    $0xd7,%esi
    cb63:	48 8d 05 56 5a 00 00 	lea    0x5a56(%rip),%rax        # 125c0 <pt_fail>
    cb6a:	ff d0                	call   *%rax
    cb6c:	0f 0b                	ud2
    cb6e:	b8 01 00 00 00       	mov    $0x1,%eax
    cb73:	48 8d 35 34 0a 04 00 	lea    0x40a34(%rip),%rsi        # 4d5ae <.Ldatac>
    cb7a:	ba 17 00 00 00       	mov    $0x17,%edx
    cb7f:	48 8d 0d bd 16 04 00 	lea    0x416bd(%rip),%rcx        # 4e243 <.Ldata27>
    cb86:	41 b8 4e 00 00 00    	mov    $0x4e,%r8d
    cb8c:	4c 8d 0d fe 16 04 00 	lea    0x416fe(%rip),%r9        # 4e291 <.Ldata28>
    cb93:	41 ba 3e 00 00 00    	mov    $0x3e,%r10d
    cb99:	48 0f b6 f8          	movzbq %al,%rdi
    cb9d:	4c 89 14 24          	mov    %r10,(%rsp)
    cba1:	48 8d 05 18 62 00 00 	lea    0x6218(%rip),%rax        # 12dc0 <pt_raise_msg>
    cba8:	ff d0                	call   *%rax
    cbaa:	e9 03 fc ff ff       	jmp    c7b2 <py1_quicksort+0x452>
    cbaf:	48 8d 3d a7 15 04 00 	lea    0x415a7(%rip),%rdi        # 4e15d <.Ldata26>
    cbb6:	be e6 00 00 00       	mov    $0xe6,%esi
    cbbb:	48 8d 0d fe 59 00 00 	lea    0x59fe(%rip),%rcx        # 125c0 <pt_fail>
    cbc2:	ff d1                	call   *%rcx
    cbc4:	0f 0b                	ud2
    cbc6:	48 8d 3d b7 14 04 00 	lea    0x414b7(%rip),%rdi        # 4e084 <.Ldata25>
    cbcd:	be d9 00 00 00       	mov    $0xd9,%esi
    cbd2:	4c 8d 05 e7 59 00 00 	lea    0x59e7(%rip),%r8        # 125c0 <pt_fail>
    cbd9:	41 ff d0             	call   *%r8
    cbdc:	0f 0b                	ud2
    cbde:	48 8d 3d cb 13 04 00 	lea    0x413cb(%rip),%rdi        # 4dfb0 <.Ldata24>
    cbe5:	be d4 00 00 00       	mov    $0xd4,%esi
    cbea:	4c 8d 05 cf 59 00 00 	lea    0x59cf(%rip),%r8        # 125c0 <pt_fail>
    cbf1:	41 ff d0             	call   *%r8
    cbf4:	0f 0b                	ud2
    cbf6:	bf 01 00 00 00       	mov    $0x1,%edi
    cbfb:	48 8d 35 ac 09 04 00 	lea    0x409ac(%rip),%rsi        # 4d5ae <.Ldatac>
    cc02:	ba 17 00 00 00       	mov    $0x17,%edx
    cc07:	48 8d 0d 09 13 04 00 	lea    0x41309(%rip),%rcx        # 4df17 <.Ldata22>
    cc0e:	41 b8 4e 00 00 00    	mov    $0x4e,%r8d
    cc14:	4c 8d 0d 4a 13 04 00 	lea    0x4134a(%rip),%r9        # 4df65 <.Ldata23>
    cc1b:	41 ba 4b 00 00 00    	mov    $0x4b,%r10d
    cc21:	48 0f b6 ff          	movzbq %dil,%rdi
    cc25:	4c 89 14 24          	mov    %r10,(%rsp)
    cc29:	4c 8d 15 90 61 00 00 	lea    0x6190(%rip),%r10        # 12dc0 <pt_raise_msg>
    cc30:	41 ff d2             	call   *%r10
    cc33:	e9 7a fb ff ff       	jmp    c7b2 <py1_quicksort+0x452>
    cc38:	48 8d 3d d7 10 04 00 	lea    0x410d7(%rip),%rdi        # 4dd16 <.Ldata1e>
    cc3f:	be b2 00 00 00       	mov    $0xb2,%esi
    cc44:	48 8d 05 75 59 00 00 	lea    0x5975(%rip),%rax        # 125c0 <pt_fail>
    cc4b:	ff d0                	call   *%rax
    cc4d:	0f 0b                	ud2

Disassembly of section .fini:

/workspace/scratch/2ce9dd46a989/card814-base/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000bd20 <pythrust_main>:
    bd20:	55                   	push   %rbp
    bd21:	48 89 e5             	mov    %rsp,%rbp
    bd24:	48 81 ec 80 00 00 00 	sub    $0x80,%rsp
    bd2b:	48 89 5c 24 50       	mov    %rbx,0x50(%rsp)
    bd30:	4c 89 64 24 58       	mov    %r12,0x58(%rsp)
    bd35:	4c 89 6c 24 60       	mov    %r13,0x60(%rsp)
    bd3a:	4c 89 74 24 68       	mov    %r14,0x68(%rsp)
    bd3f:	4c 89 7c 24 70       	mov    %r15,0x70(%rsp)
    bd44:	49 89 fd             	mov    %rdi,%r13
    bd47:	48 8d 3d b2 12 04 00 	lea    0x412b2(%rip),%rdi        # 4d000 <.Ldata0>
    bd4e:	be 02 00 00 00       	mov    $0x2,%esi
    bd53:	48 33 db             	xor    %rbx,%rbx
    bd56:	48 8d 05 c3 44 00 00 	lea    0x44c3(%rip),%rax        # 10220 <pt_seq_new>
    bd5d:	48 89 da             	mov    %rbx,%rdx
    bd60:	ff d0                	call   *%rax
    bd62:	48 89 44 24 48       	mov    %rax,0x48(%rsp)
    bd67:	b8 39 30 00 00       	mov    $0x3039,%eax
    bd6c:	48 81 fb e0 93 04 00 	cmp    $0x493e0,%rbx
    bd73:	0f 8d df 00 00 00    	jge    be58 <pythrust_main+0x138>
    bd79:	48 f7 2d c0 05 00 00 	imulq  0x5c0(%rip)        # c340 <pythrust_main+0x620>
    bd80:	0f 80 a1 05 00 00    	jo     c327 <pythrust_main+0x607>
    bd86:	48 05 39 30 00 00    	add    $0x3039,%rax
    bd8c:	0f 90 c1             	seto   %cl
    bd8f:	84 c9                	test   %cl,%cl
    bd91:	0f 85 78 05 00 00    	jne    c30f <pythrust_main+0x5ef>
    bd97:	48 89 c1             	mov    %rax,%rcx
    bd9a:	48 c1 f9 1e          	sar    $0x1e,%rcx
    bd9e:	48 c1 e9 21          	shr    $0x21,%rcx
    bda2:	48 8d 14 08          	lea    (%rax,%rcx,1),%rdx
    bda6:	48 81 e2 00 00 00 80 	and    $0xffffffff80000000,%rdx
    bdad:	48 89 c1             	mov    %rax,%rcx
    bdb0:	48 2b ca             	sub    %rdx,%rcx
    bdb3:	48 3b c2             	cmp    %rdx,%rax
    bdb6:	0f 95 c0             	setne  %al
    bdb9:	48 89 ca             	mov    %rcx,%rdx
    bdbc:	48 33 15 85 05 00 00 	xor    0x585(%rip),%rdx        # c348 <pythrust_main+0x628>
    bdc3:	48 33 ff             	xor    %rdi,%rdi
    bdc6:	48 c1 ea 3f          	shr    $0x3f,%rdx
    bdca:	84 c2                	test   %al,%dl
    bdcc:	48 89 f8             	mov    %rdi,%rax
    bdcf:	48 0f 45 05 71 05 00 	cmovne 0x571(%rip),%rax        # c348 <pythrust_main+0x628>
    bdd6:	00 
    bdd7:	48 03 c1             	add    %rcx,%rax
    bdda:	49 89 c4             	mov    %rax,%r12
    bddd:	48 f7 2d 6c 05 00 00 	imulq  0x56c(%rip)        # c350 <pythrust_main+0x630>
    bde4:	48 c1 fa 12          	sar    $0x12,%rdx
    bde8:	48 89 d0             	mov    %rdx,%rax
    bdeb:	48 c1 e8 3f          	shr    $0x3f,%rax
    bdef:	48 03 c2             	add    %rdx,%rax
    bdf2:	48 69 c8 40 42 0f 00 	imul   $0xf4240,%rax,%rcx
    bdf9:	4c 89 e0             	mov    %r12,%rax
    bdfc:	48 2b c1             	sub    %rcx,%rax
    bdff:	4c 3b e1             	cmp    %rcx,%r12
    be02:	0f 95 c1             	setne  %cl
    be05:	48 89 c2             	mov    %rax,%rdx
    be08:	48 81 f2 40 42 0f 00 	xor    $0xf4240,%rdx
    be0f:	48 c1 ea 3f          	shr    $0x3f,%rdx
    be13:	84 ca                	test   %cl,%dl
    be15:	48 0f 45 3d 3b 05 00 	cmovne 0x53b(%rip),%rdi        # c358 <pythrust_main+0x638>
    be1c:	00 
    be1d:	48 8d 34 38          	lea    (%rax,%rdi,1),%rsi
    be21:	48 8d 05 c8 44 00 00 	lea    0x44c8(%rip),%rax        # 102f0 <pt_seq_push>
    be28:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    be2d:	ff d0                	call   *%rax
    be2f:	48 89 d8             	mov    %rbx,%rax
    be32:	48 83 c0 01          	add    $0x1,%rax
    be36:	0f 90 c0             	seto   %al
    be39:	84 c0                	test   %al,%al
    be3b:	0f 85 17 00 00 00    	jne    be58 <pythrust_main+0x138>
    be41:	48 83 c3 01          	add    $0x1,%rbx
    be45:	0f 90 c0             	seto   %al
    be48:	84 c0                	test   %al,%al
    be4a:	0f 85 bd 02 00 00    	jne    c10d <pythrust_main+0x3ed>
    be50:	4c 89 e0             	mov    %r12,%rax
    be53:	e9 14 ff ff ff       	jmp    bd6c <pythrust_main+0x4c>
    be58:	48 8d 05 41 43 00 00 	lea    0x4341(%rip),%rax        # 101a0 <pt_seq_len>
    be5f:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    be64:	ff d0                	call   *%rax
    be66:	48 89 c1             	mov    %rax,%rcx
    be69:	48 83 e9 01          	sub    $0x1,%rcx
    be6d:	0f 90 c0             	seto   %al
    be70:	84 c0                	test   %al,%al
    be72:	0f 85 7f 04 00 00    	jne    c2f7 <pythrust_main+0x5d7>
    be78:	4c 89 eb             	mov    %r13,%rbx
    be7b:	4c 8b 63 08          	mov    0x8(%rbx),%r12
    be7f:	49 81 fc e8 03 00 00 	cmp    $0x3e8,%r12
    be86:	0f 8d 53 04 00 00    	jge    c2df <pythrust_main+0x5bf>
    be8c:	49 8d 44 24 01       	lea    0x1(%r12),%rax
    be91:	48 89 43 08          	mov    %rax,0x8(%rbx)
    be95:	48 33 d2             	xor    %rdx,%rdx
    be98:	48 8b 74 24 48       	mov    0x48(%rsp),%rsi
    be9d:	48 89 df             	mov    %rbx,%rdi
    bea0:	e8 bb 04 00 00       	call   c360 <py1_quicksort>
    bea5:	4c 89 63 08          	mov    %r12,0x8(%rbx)
    bea9:	48 0f b6 03          	movzbq (%rbx),%rax
    bead:	84 c0                	test   %al,%al
    beaf:	0f 85 11 04 00 00    	jne    c2c6 <pythrust_main+0x5a6>
    beb5:	48 8d 05 e4 42 00 00 	lea    0x42e4(%rip),%rax        # 101a0 <pt_seq_len>
    bebc:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bec1:	ff d0                	call   *%rax
    bec3:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
    bec8:	41 bc 01 00 00 00    	mov    $0x1,%r12d
    bece:	bb 01 00 00 00       	mov    $0x1,%ebx
    bed3:	4c 8d 6c 24 10       	lea    0x10(%rsp),%r13
    bed8:	4c 8d 74 24 18       	lea    0x18(%rsp),%r14
    bedd:	48 8b 4c 24 40       	mov    0x40(%rsp),%rcx
    bee2:	49 89 df             	mov    %rbx,%r15
    bee5:	4c 3b e1             	cmp    %rcx,%r12
    bee8:	48 89 4c 24 40       	mov    %rcx,0x40(%rsp)
    beed:	0f 8d 9d 00 00 00    	jge    bf90 <pythrust_main+0x270>
    bef3:	4c 89 e6             	mov    %r12,%rsi
    bef6:	48 83 ee 01          	sub    $0x1,%rsi
    befa:	0f 90 c0             	seto   %al
    befd:	84 c0                	test   %al,%al
    beff:	0f 85 aa 03 00 00    	jne    c2af <pythrust_main+0x58f>
    bf05:	48 8d 05 f4 41 00 00 	lea    0x41f4(%rip),%rax        # 10100 <pt_seq_get>
    bf0c:	4c 89 ea             	mov    %r13,%rdx
    bf0f:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bf14:	ff d0                	call   *%rax
    bf16:	48 89 44 24 38       	mov    %rax,0x38(%rsp)
    bf1b:	48 0f b6 44 24 10    	movzbq 0x10(%rsp),%rax
    bf21:	3c 01                	cmp    $0x1,%al
    bf23:	0f 84 45 03 00 00    	je     c26e <pythrust_main+0x54e>
    bf29:	48 8d 05 d0 41 00 00 	lea    0x41d0(%rip),%rax        # 10100 <pt_seq_get>
    bf30:	4c 89 f2             	mov    %r14,%rdx
    bf33:	4c 89 e6             	mov    %r12,%rsi
    bf36:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bf3b:	ff d0                	call   *%rax
    bf3d:	48 0f b6 4c 24 18    	movzbq 0x18(%rsp),%rcx
    bf43:	40 80 f9 01          	rex cmp $0x1,%cl
    bf47:	0f 84 e0 02 00 00    	je     c22d <pythrust_main+0x50d>
    bf4d:	48 8b 4c 24 38       	mov    0x38(%rsp),%rcx
    bf52:	48 3b c8             	cmp    %rax,%rcx
    bf55:	0f 8e 08 00 00 00    	jle    bf63 <pythrust_main+0x243>
    bf5b:	bb 01 00 00 00       	mov    $0x1,%ebx
    bf60:	45 33 ff             	xor    %r15d,%r15d
    bf63:	4c 89 e0             	mov    %r12,%rax
    bf66:	48 83 c0 01          	add    $0x1,%rax
    bf6a:	0f 90 c1             	seto   %cl
    bf6d:	84 c9                	test   %cl,%cl
    bf6f:	0f 85 1b 00 00 00    	jne    bf90 <pythrust_main+0x270>
    bf75:	49 83 c4 01          	add    $0x1,%r12
    bf79:	40 0f 90 c6          	seto   %sil
    bf7d:	40 84 f6             	test   %sil,%sil
    bf80:	0f 85 9e 01 00 00    	jne    c124 <pythrust_main+0x404>
    bf86:	48 8b 4c 24 40       	mov    0x40(%rsp),%rcx
    bf8b:	e9 55 ff ff ff       	jmp    bee5 <pythrust_main+0x1c5>
    bf90:	84 db                	test   %bl,%bl
    bf92:	0f 84 7e 02 00 00    	je     c216 <pythrust_main+0x4f6>
    bf98:	48 33 f6             	xor    %rsi,%rsi
    bf9b:	48 8d 54 24 20       	lea    0x20(%rsp),%rdx
    bfa0:	4c 8d 1d 59 41 00 00 	lea    0x4159(%rip),%r11        # 10100 <pt_seq_get>
    bfa7:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bfac:	41 ff d3             	call   *%r11
    bfaf:	49 89 c4             	mov    %rax,%r12
    bfb2:	4c 0f b6 5c 24 20    	movzbq 0x20(%rsp),%r11
    bfb8:	41 80 fb 01          	cmp    $0x1,%r11b
    bfbc:	0f 84 13 02 00 00    	je     c1d5 <pythrust_main+0x4b5>
    bfc2:	48 8d 05 d7 41 00 00 	lea    0x41d7(%rip),%rax        # 101a0 <pt_seq_len>
    bfc9:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bfce:	ff d0                	call   *%rax
    bfd0:	48 89 c1             	mov    %rax,%rcx
    bfd3:	48 c1 e9 3f          	shr    $0x3f,%rcx
    bfd7:	48 03 c8             	add    %rax,%rcx
    bfda:	48 89 ce             	mov    %rcx,%rsi
    bfdd:	48 d1 fe             	sar    $1,%rsi
    bfe0:	48 83 e1 fe          	and    $0xfffffffffffffffe,%rcx
    bfe4:	48 3b c1             	cmp    %rcx,%rax
    bfe7:	0f 95 c2             	setne  %dl
    bfea:	48 2b c1             	sub    %rcx,%rax
    bfed:	48 83 f0 02          	xor    $0x2,%rax
    bff1:	48 c1 e8 3f          	shr    $0x3f,%rax
    bff5:	23 d0                	and    %eax,%edx
    bff7:	48 0f b6 c2          	movzbq %dl,%rax
    bffb:	48 2b f0             	sub    %rax,%rsi
    bffe:	48 8d 54 24 28       	lea    0x28(%rsp),%rdx
    c003:	48 8d 05 f6 40 00 00 	lea    0x40f6(%rip),%rax        # 10100 <pt_seq_get>
    c00a:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    c00f:	ff d0                	call   *%rax
    c011:	49 89 c5             	mov    %rax,%r13
    c014:	48 0f b6 44 24 28    	movzbq 0x28(%rsp),%rax
    c01a:	3c 01                	cmp    $0x1,%al
    c01c:	0f 84 72 01 00 00    	je     c194 <pythrust_main+0x474>
    c022:	48 8d 05 77 41 00 00 	lea    0x4177(%rip),%rax        # 101a0 <pt_seq_len>
    c029:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    c02e:	ff d0                	call   *%rax
    c030:	48 89 c6             	mov    %rax,%rsi
    c033:	48 83 ee 01          	sub    $0x1,%rsi
    c037:	0f 90 c0             	seto   %al
    c03a:	84 c0                	test   %al,%al
    c03c:	0f 85 3b 01 00 00    	jne    c17d <pythrust_main+0x45d>
    c042:	48 8d 54 24 30       	lea    0x30(%rsp),%rdx
    c047:	48 8d 05 b2 40 00 00 	lea    0x40b2(%rip),%rax        # 10100 <pt_seq_get>
    c04e:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    c053:	ff d0                	call   *%rax
    c055:	49 89 c6             	mov    %rax,%r14
    c058:	48 0f b6 44 24 30    	movzbq 0x30(%rsp),%rax
    c05e:	3c 01                	cmp    $0x1,%al
    c060:	0f 84 d6 00 00 00    	je     c13c <pythrust_main+0x41c>
    c066:	49 0f b6 ff          	movzbq %r15b,%rdi
    c06a:	48 8d 05 cf 65 00 00 	lea    0x65cf(%rip),%rax        # 12640 <pt_write_bool>
    c071:	ff d0                	call   *%rax
    c073:	bb 20 00 00 00       	mov    $0x20,%ebx
    c078:	48 0f b6 fb          	movzbq %bl,%rdi
    c07c:	48 8d 05 fd 65 00 00 	lea    0x65fd(%rip),%rax        # 12680 <pt_write_byte>
    c083:	ff d0                	call   *%rax
    c085:	48 8d 05 24 66 00 00 	lea    0x6624(%rip),%rax        # 126b0 <pt_write_int>
    c08c:	4c 89 e7             	mov    %r12,%rdi
    c08f:	ff d0                	call   *%rax
    c091:	48 0f b6 fb          	movzbq %bl,%rdi
    c095:	48 8d 05 e4 65 00 00 	lea    0x65e4(%rip),%rax        # 12680 <pt_write_byte>
    c09c:	ff d0                	call   *%rax
    c09e:	48 8d 05 0b 66 00 00 	lea    0x660b(%rip),%rax        # 126b0 <pt_write_int>
    c0a5:	4c 89 ef             	mov    %r13,%rdi
    c0a8:	ff d0                	call   *%rax
    c0aa:	48 0f b6 fb          	movzbq %bl,%rdi
    c0ae:	48 8d 05 cb 65 00 00 	lea    0x65cb(%rip),%rax        # 12680 <pt_write_byte>
    c0b5:	ff d0                	call   *%rax
    c0b7:	48 8d 05 f2 65 00 00 	lea    0x65f2(%rip),%rax        # 126b0 <pt_write_int>
    c0be:	4c 89 f7             	mov    %r14,%rdi
    c0c1:	ff d0                	call   *%rax
    c0c3:	b8 0a 00 00 00       	mov    $0xa,%eax
    c0c8:	48 0f b6 f8          	movzbq %al,%rdi
    c0cc:	48 8d 05 ad 65 00 00 	lea    0x65ad(%rip),%rax        # 12680 <pt_write_byte>
    c0d3:	ff d0                	call   *%rax
    c0d5:	48 8d 05 44 3f 00 00 	lea    0x3f44(%rip),%rax        # 10020 <pt_seq_dec>
    c0dc:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    c0e1:	ff d0                	call   *%rax
    c0e3:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
    c0e8:	4c 8b 64 24 58       	mov    0x58(%rsp),%r12
    c0ed:	4c 8b 6c 24 60       	mov    0x60(%rsp),%r13
    c0f2:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
    c0f7:	4c 8b 7c 24 70       	mov    0x70(%rsp),%r15
    c0fc:	48 81 c4 80 00 00 00 	add    $0x80,%rsp
    c103:	48 89 ec             	mov    %rbp,%rsp
    c106:	5d                   	pop    %rbp
    c107:	c3                   	ret
    c108:	e9 b9 01 00 00       	jmp    c2c6 <pythrust_main+0x5a6>
    c10d:	48 8d 3d ee 11 04 00 	lea    0x411ee(%rip),%rdi        # 4d302 <.Ldata8>
    c114:	be a2 00 00 00       	mov    $0xa2,%esi
    c119:	48 8d 05 a0 64 00 00 	lea    0x64a0(%rip),%rax        # 125c0 <pt_fail>
    c120:	ff d0                	call   *%rax
    c122:	0f 0b                	ud2
    c124:	48 8d 3d 18 15 04 00 	lea    0x41518(%rip),%rdi        # 4d643 <.Ldataf>
    c12b:	be aa 00 00 00       	mov    $0xaa,%esi
    c130:	4c 8d 05 89 64 00 00 	lea    0x6489(%rip),%r8        # 125c0 <pt_fail>
    c137:	41 ff d0             	call   *%r8
    c13a:	0f 0b                	ud2
    c13c:	b8 01 00 00 00       	mov    $0x1,%eax
    c141:	48 8d 35 66 14 04 00 	lea    0x41466(%rip),%rsi        # 4d5ae <.Ldatac>
    c148:	ba 17 00 00 00       	mov    $0x17,%edx
    c14d:	48 8d 0d 07 1b 04 00 	lea    0x41b07(%rip),%rcx        # 4dc5b <.Ldata1c>
    c154:	41 b8 4f 00 00 00    	mov    $0x4f,%r8d
    c15a:	4c 8d 0d 49 1b 04 00 	lea    0x41b49(%rip),%r9        # 4dcaa <.Ldata1d>
    c161:	41 ba 6c 00 00 00    	mov    $0x6c,%r10d
    c167:	48 0f b6 f8          	movzbq %al,%rdi
    c16b:	4c 89 14 24          	mov    %r10,(%rsp)
    c16f:	48 8d 05 4a 6c 00 00 	lea    0x6c4a(%rip),%rax        # 12dc0 <pt_raise_msg>
    c176:	ff d0                	call   *%rax
    c178:	e9 49 01 00 00       	jmp    c2c6 <pythrust_main+0x5a6>
    c17d:	48 8d 3d fe 19 04 00 	lea    0x419fe(%rip),%rdi        # 4db82 <.Ldata1b>
    c184:	be d9 00 00 00       	mov    $0xd9,%esi
    c189:	48 8d 05 30 64 00 00 	lea    0x6430(%rip),%rax        # 125c0 <pt_fail>
    c190:	ff d0                	call   *%rax
    c192:	0f 0b                	ud2
    c194:	b8 01 00 00 00       	mov    $0x1,%eax
    c199:	48 8d 35 0e 14 04 00 	lea    0x4140e(%rip),%rsi        # 4d5ae <.Ldatac>
    c1a0:	ba 17 00 00 00       	mov    $0x17,%edx
    c1a5:	48 8d 0d 2c 19 04 00 	lea    0x4192c(%rip),%rcx        # 4dad8 <.Ldata19>
    c1ac:	41 b8 4f 00 00 00    	mov    $0x4f,%r8d
    c1b2:	4c 8d 0d 6e 19 04 00 	lea    0x4196e(%rip),%r9        # 4db27 <.Ldata1a>
    c1b9:	41 ba 5b 00 00 00    	mov    $0x5b,%r10d
    c1bf:	48 0f b6 f8          	movzbq %al,%rdi
    c1c3:	4c 89 14 24          	mov    %r10,(%rsp)
    c1c7:	48 8d 05 f2 6b 00 00 	lea    0x6bf2(%rip),%rax        # 12dc0 <pt_raise_msg>
    c1ce:	ff d0                	call   *%rax
    c1d0:	e9 f1 00 00 00       	jmp    c2c6 <pythrust_main+0x5a6>
    c1d5:	b8 01 00 00 00       	mov    $0x1,%eax
    c1da:	48 8d 35 cd 13 04 00 	lea    0x413cd(%rip),%rsi        # 4d5ae <.Ldatac>
    c1e1:	ba 17 00 00 00       	mov    $0x17,%edx
    c1e6:	48 8d 0d e2 16 04 00 	lea    0x416e2(%rip),%rcx        # 4d8cf <.Ldata14>
    c1ed:	41 b8 4f 00 00 00    	mov    $0x4f,%r8d
    c1f3:	4c 8d 0d 24 17 04 00 	lea    0x41724(%rip),%r9        # 4d91e <.Ldata15>
    c1fa:	41 ba 49 00 00 00    	mov    $0x49,%r10d
    c200:	48 0f b6 f8          	movzbq %al,%rdi
    c204:	4c 89 14 24          	mov    %r10,(%rsp)
    c208:	48 8d 05 b1 6b 00 00 	lea    0x6bb1(%rip),%rax        # 12dc0 <pt_raise_msg>
    c20f:	ff d0                	call   *%rax
    c211:	e9 b0 00 00 00       	jmp    c2c6 <pythrust_main+0x5a6>
    c216:	48 8d 3d ff 15 04 00 	lea    0x415ff(%rip),%rdi        # 4d81c <.Ldata13>
    c21d:	be b3 00 00 00       	mov    $0xb3,%esi
    c222:	48 8d 05 97 63 00 00 	lea    0x6397(%rip),%rax        # 125c0 <pt_fail>
    c229:	ff d0                	call   *%rax
    c22b:	0f 0b                	ud2
    c22d:	b8 01 00 00 00       	mov    $0x1,%eax
    c232:	48 8d 35 75 13 04 00 	lea    0x41375(%rip),%rsi        # 4d5ae <.Ldatac>
    c239:	ba 17 00 00 00       	mov    $0x17,%edx
    c23e:	48 8d 0d 50 15 04 00 	lea    0x41550(%rip),%rcx        # 4d795 <.Ldata11>
    c245:	41 b8 4f 00 00 00    	mov    $0x4f,%r8d
    c24b:	4c 8d 0d 92 15 04 00 	lea    0x41592(%rip),%r9        # 4d7e4 <.Ldata12>
    c252:	41 ba 38 00 00 00    	mov    $0x38,%r10d
    c258:	48 0f b6 f8          	movzbq %al,%rdi
    c25c:	4c 89 14 24          	mov    %r10,(%rsp)
    c260:	48 8d 05 59 6b 00 00 	lea    0x6b59(%rip),%rax        # 12dc0 <pt_raise_msg>
    c267:	ff d0                	call   *%rax
    c269:	e9 58 00 00 00       	jmp    c2c6 <pythrust_main+0x5a6>
    c26e:	b8 01 00 00 00       	mov    $0x1,%eax
    c273:	48 8d 35 34 13 04 00 	lea    0x41334(%rip),%rsi        # 4d5ae <.Ldatac>
    c27a:	ba 17 00 00 00       	mov    $0x17,%edx
    c27f:	48 8d 0d 3f 13 04 00 	lea    0x4133f(%rip),%rcx        # 4d5c5 <.Ldatad>
    c286:	41 b8 4e 00 00 00    	mov    $0x4e,%r8d
    c28c:	4c 8d 0d 80 13 04 00 	lea    0x41380(%rip),%r9        # 4d613 <.Ldatae>
    c293:	41 ba 30 00 00 00    	mov    $0x30,%r10d
    c299:	48 0f b6 f8          	movzbq %al,%rdi
    c29d:	4c 89 14 24          	mov    %r10,(%rsp)
    c2a1:	48 8d 05 18 6b 00 00 	lea    0x6b18(%rip),%rax        # 12dc0 <pt_raise_msg>
    c2a8:	ff d0                	call   *%rax
    c2aa:	e9 17 00 00 00       	jmp    c2c6 <pythrust_main+0x5a6>
    c2af:	48 8d 3d 5b 12 04 00 	lea    0x4125b(%rip),%rdi        # 4d511 <.Ldatab>
    c2b6:	be 9d 00 00 00       	mov    $0x9d,%esi
    c2bb:	48 8d 05 fe 62 00 00 	lea    0x62fe(%rip),%rax        # 125c0 <pt_fail>
    c2c2:	ff d0                	call   *%rax
    c2c4:	0f 0b                	ud2
    c2c6:	48 8d 05 53 3d 00 00 	lea    0x3d53(%rip),%rax        # 10020 <pt_seq_dec>
    c2cd:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    c2d2:	ff d0                	call   *%rax
    c2d4:	48 8d 0d d5 6b 00 00 	lea    0x6bd5(%rip),%rcx        # 12eb0 <pt_uncaught>
    c2db:	ff d1                	call   *%rcx
    c2dd:	0f 0b                	ud2
    c2df:	48 8d 3d 6c 11 04 00 	lea    0x4116c(%rip),%rdi        # 4d452 <.Ldataa>
    c2e6:	be bf 00 00 00       	mov    $0xbf,%esi
    c2eb:	4c 8d 05 ce 62 00 00 	lea    0x62ce(%rip),%r8        # 125c0 <pt_fail>
    c2f2:	41 ff d0             	call   *%r8
    c2f5:	0f 0b                	ud2
    c2f7:	48 8d 3d a6 10 04 00 	lea    0x410a6(%rip),%rdi        # 4d3a4 <.Ldata9>
    c2fe:	be ae 00 00 00       	mov    $0xae,%esi
    c303:	4c 8d 05 b6 62 00 00 	lea    0x62b6(%rip),%r8        # 125c0 <pt_fail>
    c30a:	41 ff d0             	call   *%r8
    c30d:	0f 0b                	ud2
    c30f:	48 8d 3d b1 0d 04 00 	lea    0x40db1(%rip),%rdi        # 4d0c7 <.Ldata2>
    c316:	be ce 00 00 00       	mov    $0xce,%esi
    c31b:	4c 8d 05 9e 62 00 00 	lea    0x629e(%rip),%r8        # 125c0 <pt_fail>
    c322:	41 ff d0             	call   *%r8
    c325:	0f 0b                	ud2
    c327:	48 8d 3d d4 0c 04 00 	lea    0x40cd4(%rip),%rdi        # 4d002 <.Ldata1>
    c32e:	be c5 00 00 00       	mov    $0xc5,%esi
    c333:	4c 8d 0d 86 62 00 00 	lea    0x6286(%rip),%r9        # 125c0 <pt_fail>
    c33a:	41 ff d1             	call   *%r9
    c33d:	0f 0b                	ud2
    c33f:	00 6d 4e             	add    %ch,0x4e(%rbp)
    c342:	c6 41 00 00          	movb   $0x0,0x0(%rcx)
    c346:	00 00                	add    %al,(%rax)
    c348:	00 00                	add    %al,(%rax)
    c34a:	00 80 00 00 00 00    	add    %al,0x0(%rax)
    c350:	db 34 b6             	(bad) (%rsi,%rsi,4)
    c353:	d7                   	xlat   %ds:(%rbx)
    c354:	82                   	(bad)
    c355:	de 1b                	ficomps (%rbx)
    c357:	43                   	rex.XB
    c358:	40                   	rex
    c359:	42 0f 00 00          	rex.X sldt (%rax)
    c35d:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:
