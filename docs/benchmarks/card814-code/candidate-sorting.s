
/workspace/scratch/2ce9dd46a989/card814-candidate-final/sorting:     file format elf64-x86-64


Disassembly of section .init:

Disassembly of section .plt:

Disassembly of section .plt.got:

Disassembly of section .text:

000000000000c340 <py1_quicksort>:
    c340:	55                   	push   %rbp
    c341:	48 89 e5             	mov    %rsp,%rbp
    c344:	48 81 ec b0 00 00 00 	sub    $0xb0,%rsp
    c34b:	48 89 9c 24 80 00 00 	mov    %rbx,0x80(%rsp)
    c352:	00 
    c353:	4c 89 a4 24 88 00 00 	mov    %r12,0x88(%rsp)
    c35a:	00 
    c35b:	4c 89 ac 24 90 00 00 	mov    %r13,0x90(%rsp)
    c362:	00 
    c363:	4c 89 b4 24 98 00 00 	mov    %r14,0x98(%rsp)
    c36a:	00 
    c36b:	4c 89 bc 24 a0 00 00 	mov    %r15,0xa0(%rsp)
    c372:	00 
    c373:	49 89 ff             	mov    %rdi,%r15
    c376:	48 89 74 24 38       	mov    %rsi,0x38(%rsp)
    c37b:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c380:	48 89 4c 24 58       	mov    %rcx,0x58(%rsp)
    c385:	48 8d 4c 24 10       	lea    0x10(%rsp),%rcx
    c38a:	48 89 4c 24 78       	mov    %rcx,0x78(%rsp)
    c38f:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c394:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c399:	48 3b d1             	cmp    %rcx,%rdx
    c39c:	0f 8d c0 03 00 00    	jge    c762 <py1_quicksort+0x422>
    c3a2:	48 89 d6             	mov    %rdx,%rsi
    c3a5:	48 03 f1             	add    %rcx,%rsi
    c3a8:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c3ad:	48 89 4c 24 58       	mov    %rcx,0x58(%rsp)
    c3b2:	0f 90 c0             	seto   %al
    c3b5:	84 c0                	test   %al,%al
    c3b7:	0f 85 2b 08 00 00    	jne    cbe8 <py1_quicksort+0x8a8>
    c3bd:	48 d1 fe             	sar    $1,%rsi
    c3c0:	48 8d 05 e9 3c 00 00 	lea    0x3ce9(%rip),%rax        # 100b0 <pt_seq_get>
    c3c7:	48 8b 54 24 78       	mov    0x78(%rsp),%rdx
    c3cc:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c3d1:	ff d0                	call   *%rax
    c3d3:	48 89 44 24 70       	mov    %rax,0x70(%rsp)
    c3d8:	48 0f b6 44 24 10    	movzbq 0x10(%rsp),%rax
    c3de:	3c 01                	cmp    $0x1,%al
    c3e0:	0f 84 c1 07 00 00    	je     cba7 <py1_quicksort+0x867>
    c3e6:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c3ec:	48 8d 5c 24 28       	lea    0x28(%rsp),%rbx
    c3f1:	48 89 5c 24 68       	mov    %rbx,0x68(%rsp)
    c3f6:	4c 8d 64 24 30       	lea    0x30(%rsp),%r12
    c3fb:	4c 89 64 24 60       	mov    %r12,0x60(%rsp)
    c400:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c405:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c40a:	4d 89 f5             	mov    %r14,%r13
    c40d:	45 84 f6             	test   %r14b,%r14b
    c410:	0f 84 7a 07 00 00    	je     cb90 <py1_quicksort+0x850>
    c416:	45 84 ed             	test   %r13b,%r13b
    c419:	0f 84 5a 07 00 00    	je     cb79 <py1_quicksort+0x839>
    c41f:	48 8b 44 24 40       	mov    0x40(%rsp),%rax
    c424:	48 3b c1             	cmp    %rcx,%rax
    c427:	0f 8e 21 01 00 00    	jle    c54e <py1_quicksort+0x20e>
    c42d:	45 84 ed             	test   %r13b,%r13b
    c430:	0f 84 2f 04 00 00    	je     c865 <py1_quicksort+0x525>
    c436:	48 89 c8             	mov    %rcx,%rax
    c439:	48 2b c2             	sub    %rdx,%rax
    c43c:	48 89 4c 24 48       	mov    %rcx,0x48(%rsp)
    c441:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
    c446:	0f 90 c1             	seto   %cl
    c449:	84 c9                	test   %cl,%cl
    c44b:	0f 85 fd 03 00 00    	jne    c84e <py1_quicksort+0x50e>
    c451:	45 84 f6             	test   %r14b,%r14b
    c454:	0f 84 dd 03 00 00    	je     c837 <py1_quicksort+0x4f7>
    c45a:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c45f:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c464:	48 2b ca             	sub    %rdx,%rcx
    c467:	0f 90 c2             	seto   %dl
    c46a:	84 d2                	test   %dl,%dl
    c46c:	0f 85 ae 03 00 00    	jne    c820 <py1_quicksort+0x4e0>
    c472:	48 3b c1             	cmp    %rcx,%rax
    c475:	0f 8c 72 00 00 00    	jl     c4ed <py1_quicksort+0x1ad>
    c47b:	45 84 f6             	test   %r14b,%r14b
    c47e:	0f 84 40 03 00 00    	je     c7c4 <py1_quicksort+0x484>
    c484:	4d 89 fc             	mov    %r15,%r12
    c487:	4d 8b 74 24 08       	mov    0x8(%r12),%r14
    c48c:	49 81 fe e8 03 00 00 	cmp    $0x3e8,%r14
    c493:	0f 8d 14 03 00 00    	jge    c7ad <py1_quicksort+0x46d>
    c499:	4d 8d 4e 01          	lea    0x1(%r14),%r9
    c49d:	4d 89 4c 24 08       	mov    %r9,0x8(%r12)
    c4a2:	48 8b 4c 24 58       	mov    0x58(%rsp),%rcx
    c4a7:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c4ac:	48 8b 74 24 38       	mov    0x38(%rsp),%rsi
    c4b1:	4c 89 e7             	mov    %r12,%rdi
    c4b4:	e8 87 fe ff ff       	call   c340 <py1_quicksort>
    c4b9:	4d 89 74 24 08       	mov    %r14,0x8(%r12)
    c4be:	49 0f b6 04 24       	movzbq (%r12),%rax
    c4c3:	84 c0                	test   %al,%al
    c4c5:	0f 85 97 02 00 00    	jne    c762 <py1_quicksort+0x422>
    c4cb:	45 84 ed             	test   %r13b,%r13b
    c4ce:	0f 84 c2 02 00 00    	je     c796 <py1_quicksort+0x456>
    c4d4:	48 8b 44 24 50       	mov    0x50(%rsp),%rax
    c4d9:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
    c4de:	48 8b 44 24 48       	mov    0x48(%rsp),%rax
    c4e3:	48 89 44 24 58       	mov    %rax,0x58(%rsp)
    c4e8:	e9 59 00 00 00       	jmp    c546 <py1_quicksort+0x206>
    c4ed:	4d 89 fc             	mov    %r15,%r12
    c4f0:	45 84 ed             	test   %r13b,%r13b
    c4f3:	0f 84 10 03 00 00    	je     c809 <py1_quicksort+0x4c9>
    c4f9:	49 8b 5c 24 08       	mov    0x8(%r12),%rbx
    c4fe:	48 81 fb e8 03 00 00 	cmp    $0x3e8,%rbx
    c505:	0f 8d e7 02 00 00    	jge    c7f2 <py1_quicksort+0x4b2>
    c50b:	48 8d 43 01          	lea    0x1(%rbx),%rax
    c50f:	49 89 44 24 08       	mov    %rax,0x8(%r12)
    c514:	48 8b 4c 24 48       	mov    0x48(%rsp),%rcx
    c519:	48 8b 54 24 50       	mov    0x50(%rsp),%rdx
    c51e:	48 8b 74 24 38       	mov    0x38(%rsp),%rsi
    c523:	4c 89 e7             	mov    %r12,%rdi
    c526:	e8 15 fe ff ff       	call   c340 <py1_quicksort>
    c52b:	49 89 5c 24 08       	mov    %rbx,0x8(%r12)
    c530:	49 0f b6 04 24       	movzbq (%r12),%rax
    c535:	84 c0                	test   %al,%al
    c537:	0f 85 25 02 00 00    	jne    c762 <py1_quicksort+0x422>
    c53d:	45 84 f6             	test   %r14b,%r14b
    c540:	0f 84 95 02 00 00    	je     c7db <py1_quicksort+0x49b>
    c546:	4d 89 e7             	mov    %r12,%r15
    c549:	e9 41 fe ff ff       	jmp    c38f <py1_quicksort+0x4f>
    c54e:	4d 89 fc             	mov    %r15,%r12
    c551:	48 89 4c 24 48       	mov    %rcx,0x48(%rsp)
    c556:	48 89 54 24 50       	mov    %rdx,0x50(%rsp)
    c55b:	48 8d 5c 24 18       	lea    0x18(%rsp),%rbx
    c560:	45 84 f6             	test   %r14b,%r14b
    c563:	0f 84 f9 05 00 00    	je     cb62 <py1_quicksort+0x822>
    c569:	48 8d 05 40 3b 00 00 	lea    0x3b40(%rip),%rax        # 100b0 <pt_seq_get>
    c570:	48 89 da             	mov    %rbx,%rdx
    c573:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c578:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c57d:	ff d0                	call   *%rax
    c57f:	48 0f b6 4c 24 18    	movzbq 0x18(%rsp),%rcx
    c585:	40 80 f9 01          	rex cmp $0x1,%cl
    c589:	0f 84 92 05 00 00    	je     cb21 <py1_quicksort+0x7e1>
    c58f:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
    c594:	48 3b c1             	cmp    %rcx,%rax
    c597:	0f 8c 8e 01 00 00    	jl     c72b <py1_quicksort+0x3eb>
    c59d:	48 8d 5c 24 20       	lea    0x20(%rsp),%rbx
    c5a2:	45 84 ed             	test   %r13b,%r13b
    c5a5:	0f 84 31 05 00 00    	je     cadc <py1_quicksort+0x79c>
    c5ab:	4c 8d 05 fe 3a 00 00 	lea    0x3afe(%rip),%r8        # 100b0 <pt_seq_get>
    c5b2:	4c 8b 7c 24 48       	mov    0x48(%rsp),%r15
    c5b7:	48 89 da             	mov    %rbx,%rdx
    c5ba:	4c 89 fe             	mov    %r15,%rsi
    c5bd:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c5c2:	41 ff d0             	call   *%r8
    c5c5:	48 0f b6 74 24 20    	movzbq 0x20(%rsp),%rsi
    c5cb:	40 80 fe 01          	cmp    $0x1,%sil
    c5cf:	0f 84 c6 04 00 00    	je     ca9b <py1_quicksort+0x75b>
    c5d5:	48 8b 4c 24 70       	mov    0x70(%rsp),%rcx
    c5da:	48 3b c1             	cmp    %rcx,%rax
    c5dd:	0f 8f 20 01 00 00    	jg     c703 <py1_quicksort+0x3c3>
    c5e3:	45 84 f6             	test   %r14b,%r14b
    c5e6:	0f 84 6a 04 00 00    	je     ca56 <py1_quicksort+0x716>
    c5ec:	45 84 ed             	test   %r13b,%r13b
    c5ef:	0f 84 4a 04 00 00    	je     ca3f <py1_quicksort+0x6ff>
    c5f5:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c5fa:	49 3b d7             	cmp    %r15,%rdx
    c5fd:	0f 8f f0 00 00 00    	jg     c6f3 <py1_quicksort+0x3b3>
    c603:	45 84 f6             	test   %r14b,%r14b
    c606:	0f 84 1c 04 00 00    	je     ca28 <py1_quicksort+0x6e8>
    c60c:	48 8d 05 9d 3a 00 00 	lea    0x3a9d(%rip),%rax        # 100b0 <pt_seq_get>
    c613:	48 8b 54 24 68       	mov    0x68(%rsp),%rdx
    c618:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c61d:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c622:	ff d0                	call   *%rax
    c624:	48 89 c3             	mov    %rax,%rbx
    c627:	48 0f b6 44 24 28    	movzbq 0x28(%rsp),%rax
    c62d:	3c 01                	cmp    $0x1,%al
    c62f:	0f 84 b2 03 00 00    	je     c9e7 <py1_quicksort+0x6a7>
    c635:	45 84 ed             	test   %r13b,%r13b
    c638:	0f 84 91 03 00 00    	je     c9cf <py1_quicksort+0x68f>
    c63e:	48 8d 05 6b 3a 00 00 	lea    0x3a6b(%rip),%rax        # 100b0 <pt_seq_get>
    c645:	48 8b 54 24 60       	mov    0x60(%rsp),%rdx
    c64a:	4c 89 fe             	mov    %r15,%rsi
    c64d:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c652:	ff d0                	call   *%rax
    c654:	48 0f b6 4c 24 30    	movzbq 0x30(%rsp),%rcx
    c65a:	40 80 f9 01          	rex cmp $0x1,%cl
    c65e:	0f 84 29 03 00 00    	je     c98d <py1_quicksort+0x64d>
    c664:	45 84 f6             	test   %r14b,%r14b
    c667:	0f 84 08 03 00 00    	je     c975 <py1_quicksort+0x635>
    c66d:	48 8d 0d 1c 37 00 00 	lea    0x371c(%rip),%rcx        # fd90 <pt_list_set>
    c674:	48 89 c2             	mov    %rax,%rdx
    c677:	48 8b 74 24 40       	mov    0x40(%rsp),%rsi
    c67c:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c681:	ff d1                	call   *%rcx
    c683:	3c 01                	cmp    $0x1,%al
    c685:	0f 84 a5 02 00 00    	je     c930 <py1_quicksort+0x5f0>
    c68b:	45 84 ed             	test   %r13b,%r13b
    c68e:	0f 84 85 02 00 00    	je     c919 <py1_quicksort+0x5d9>
    c694:	48 8d 05 f5 36 00 00 	lea    0x36f5(%rip),%rax        # fd90 <pt_list_set>
    c69b:	48 89 da             	mov    %rbx,%rdx
    c69e:	4c 89 fe             	mov    %r15,%rsi
    c6a1:	48 8b 7c 24 38       	mov    0x38(%rsp),%rdi
    c6a6:	ff d0                	call   *%rax
    c6a8:	3c 01                	cmp    $0x1,%al
    c6aa:	0f 84 28 02 00 00    	je     c8d8 <py1_quicksort+0x598>
    c6b0:	45 84 f6             	test   %r14b,%r14b
    c6b3:	0f 84 08 02 00 00    	je     c8c1 <py1_quicksort+0x581>
    c6b9:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c6be:	48 83 c2 01          	add    $0x1,%rdx
    c6c2:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c6c7:	0f 90 c0             	seto   %al
    c6ca:	84 c0                	test   %al,%al
    c6cc:	0f 85 d8 01 00 00    	jne    c8aa <py1_quicksort+0x56a>
    c6d2:	45 84 ed             	test   %r13b,%r13b
    c6d5:	0f 84 b8 01 00 00    	je     c893 <py1_quicksort+0x553>
    c6db:	49 83 ef 01          	sub    $0x1,%r15
    c6df:	0f 90 c0             	seto   %al
    c6e2:	84 c0                	test   %al,%al
    c6e4:	0f 85 92 01 00 00    	jne    c87c <py1_quicksort+0x53c>
    c6ea:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c6f0:	4d 89 f5             	mov    %r14,%r13
    c6f3:	48 8b 54 24 50       	mov    0x50(%rsp),%rdx
    c6f8:	4c 89 f9             	mov    %r15,%rcx
    c6fb:	4d 89 e7             	mov    %r12,%r15
    c6fe:	e9 0a fd ff ff       	jmp    c40d <py1_quicksort+0xcd>
    c703:	45 84 ed             	test   %r13b,%r13b
    c706:	0f 84 78 03 00 00    	je     ca84 <py1_quicksort+0x744>
    c70c:	49 83 ef 01          	sub    $0x1,%r15
    c710:	0f 90 c0             	seto   %al
    c713:	84 c0                	test   %al,%al
    c715:	0f 85 52 03 00 00    	jne    ca6d <py1_quicksort+0x72d>
    c71b:	41 bd 01 00 00 00    	mov    $0x1,%r13d
    c721:	4c 89 7c 24 48       	mov    %r15,0x48(%rsp)
    c726:	e9 77 fe ff ff       	jmp    c5a2 <py1_quicksort+0x262>
    c72b:	4c 8b 7c 24 48       	mov    0x48(%rsp),%r15
    c730:	45 84 f6             	test   %r14b,%r14b
    c733:	0f 84 d1 03 00 00    	je     cb0a <py1_quicksort+0x7ca>
    c739:	48 8b 54 24 40       	mov    0x40(%rsp),%rdx
    c73e:	48 83 c2 01          	add    $0x1,%rdx
    c742:	48 89 54 24 40       	mov    %rdx,0x40(%rsp)
    c747:	0f 90 c0             	seto   %al
    c74a:	84 c0                	test   %al,%al
    c74c:	0f 85 a1 03 00 00    	jne    caf3 <py1_quicksort+0x7b3>
    c752:	41 be 01 00 00 00    	mov    $0x1,%r14d
    c758:	4c 89 7c 24 48       	mov    %r15,0x48(%rsp)
    c75d:	e9 fe fd ff ff       	jmp    c560 <py1_quicksort+0x220>
    c762:	48 8b 9c 24 80 00 00 	mov    0x80(%rsp),%rbx
    c769:	00 
    c76a:	4c 8b a4 24 88 00 00 	mov    0x88(%rsp),%r12
    c771:	00 
    c772:	4c 8b ac 24 90 00 00 	mov    0x90(%rsp),%r13
    c779:	00 
    c77a:	4c 8b b4 24 98 00 00 	mov    0x98(%rsp),%r14
    c781:	00 
    c782:	4c 8b bc 24 a0 00 00 	mov    0xa0(%rsp),%r15
    c789:	00 
    c78a:	48 81 c4 b0 00 00 00 	add    $0xb0,%rsp
    c791:	48 89 ec             	mov    %rbp,%rsp
    c794:	5d                   	pop    %rbp
    c795:	c3                   	ret
    c796:	48 8d 3d ba 33 04 00 	lea    0x433ba(%rip),%rdi        # 4fb57 <.Ldata49>
    c79d:	be e0 00 00 00       	mov    $0xe0,%esi
    c7a2:	48 8d 05 c7 5d 00 00 	lea    0x5dc7(%rip),%rax        # 12570 <pt_fail>
    c7a9:	ff d0                	call   *%rax
    c7ab:	0f 0b                	ud2
    c7ad:	48 8d 3d d2 32 04 00 	lea    0x432d2(%rip),%rdi        # 4fa86 <.Ldata48>
    c7b4:	be d1 00 00 00       	mov    $0xd1,%esi
    c7b9:	48 8d 05 b0 5d 00 00 	lea    0x5db0(%rip),%rax        # 12570 <pt_fail>
    c7c0:	ff d0                	call   *%rax
    c7c2:	0f 0b                	ud2
    c7c4:	48 8d 3d c4 31 04 00 	lea    0x431c4(%rip),%rdi        # 4f98f <.Ldata47>
    c7cb:	be f7 00 00 00       	mov    $0xf7,%esi
    c7d0:	48 8d 05 99 5d 00 00 	lea    0x5d99(%rip),%rax        # 12570 <pt_fail>
    c7d7:	ff d0                	call   *%rax
    c7d9:	0f 0b                	ud2
    c7db:	48 8d 3d cd 30 04 00 	lea    0x430cd(%rip),%rdi        # 4f8af <.Ldata46>
    c7e2:	be e0 00 00 00       	mov    $0xe0,%esi
    c7e7:	48 8d 05 82 5d 00 00 	lea    0x5d82(%rip),%rax        # 12570 <pt_fail>
    c7ee:	ff d0                	call   *%rax
    c7f0:	0f 0b                	ud2
    c7f2:	48 8d 3d e5 2f 04 00 	lea    0x42fe5(%rip),%rdi        # 4f7de <.Ldata45>
    c7f9:	be d1 00 00 00       	mov    $0xd1,%esi
    c7fe:	48 8d 05 6b 5d 00 00 	lea    0x5d6b(%rip),%rax        # 12570 <pt_fail>
    c805:	ff d0                	call   *%rax
    c807:	0f 0b                	ud2
    c809:	48 8d 3d d3 2e 04 00 	lea    0x42ed3(%rip),%rdi        # 4f6e3 <.Ldata44>
    c810:	be fb 00 00 00       	mov    $0xfb,%esi
    c815:	48 8d 05 54 5d 00 00 	lea    0x5d54(%rip),%rax        # 12570 <pt_fail>
    c81c:	ff d0                	call   *%rax
    c81e:	0f 0b                	ud2
    c820:	48 8d 3d 07 2e 04 00 	lea    0x42e07(%rip),%rdi        # 4f62e <.Ldata43>
    c827:	be b5 00 00 00       	mov    $0xb5,%esi
    c82c:	48 8d 05 3d 5d 00 00 	lea    0x5d3d(%rip),%rax        # 12570 <pt_fail>
    c833:	ff d0                	call   *%rax
    c835:	0f 0b                	ud2
    c837:	48 8d 3d ff 2c 04 00 	lea    0x42cff(%rip),%rdi        # 4f53d <.Ldata42>
    c83e:	be f1 00 00 00       	mov    $0xf1,%esi
    c843:	48 8d 05 26 5d 00 00 	lea    0x5d26(%rip),%rax        # 12570 <pt_fail>
    c84a:	ff d0                	call   *%rax
    c84c:	0f 0b                	ud2
    c84e:	48 8d 3d 3c 2c 04 00 	lea    0x42c3c(%rip),%rdi        # 4f491 <.Ldata41>
    c855:	be ac 00 00 00       	mov    $0xac,%esi
    c85a:	48 8d 05 0f 5d 00 00 	lea    0x5d0f(%rip),%rax        # 12570 <pt_fail>
    c861:	ff d0                	call   *%rax
    c863:	0f 0b                	ud2
    c865:	48 8d 3d 42 2b 04 00 	lea    0x42b42(%rip),%rdi        # 4f3ae <.Ldata40>
    c86c:	be e3 00 00 00       	mov    $0xe3,%esi
    c871:	48 8d 05 f8 5c 00 00 	lea    0x5cf8(%rip),%rax        # 12570 <pt_fail>
    c878:	ff d0                	call   *%rax
    c87a:	0f 0b                	ud2
    c87c:	48 8d 3d 7f 2a 04 00 	lea    0x42a7f(%rip),%rdi        # 4f302 <.Ldata3f>
    c883:	be ac 00 00 00       	mov    $0xac,%esi
    c888:	48 8d 05 e1 5c 00 00 	lea    0x5ce1(%rip),%rax        # 12570 <pt_fail>
    c88f:	ff d0                	call   *%rax
    c891:	0f 0b                	ud2
    c893:	48 8d 3d 85 29 04 00 	lea    0x42985(%rip),%rdi        # 4f21f <.Ldata3e>
    c89a:	be e3 00 00 00       	mov    $0xe3,%esi
    c89f:	48 8d 05 ca 5c 00 00 	lea    0x5cca(%rip),%rax        # 12570 <pt_fail>
    c8a6:	ff d0                	call   *%rax
    c8a8:	0f 0b                	ud2
    c8aa:	48 8d 3d c2 28 04 00 	lea    0x428c2(%rip),%rdi        # 4f173 <.Ldata3d>
    c8b1:	be ac 00 00 00       	mov    $0xac,%esi
    c8b6:	48 8d 05 b3 5c 00 00 	lea    0x5cb3(%rip),%rax        # 12570 <pt_fail>
    c8bd:	ff d0                	call   *%rax
    c8bf:	0f 0b                	ud2
    c8c1:	48 8d 3d c8 27 04 00 	lea    0x427c8(%rip),%rdi        # 4f090 <.Ldata3c>
    c8c8:	be e3 00 00 00       	mov    $0xe3,%esi
    c8cd:	48 8d 05 9c 5c 00 00 	lea    0x5c9c(%rip),%rax        # 12570 <pt_fail>
    c8d4:	ff d0                	call   *%rax
    c8d6:	0f 0b                	ud2
    c8d8:	b8 01 00 00 00       	mov    $0x1,%eax
    c8dd:	48 8d 35 91 24 04 00 	lea    0x42491(%rip),%rsi        # 4ed75 <.Ldata35>
    c8e4:	ba 22 00 00 00       	mov    $0x22,%edx
    c8e9:	48 8d 0d 11 27 04 00 	lea    0x42711(%rip),%rcx        # 4f001 <.Ldata3a>
    c8f0:	41 b8 5a 00 00 00    	mov    $0x5a,%r8d
    c8f6:	4c 8d 0d 5e 27 04 00 	lea    0x4275e(%rip),%r9        # 4f05b <.Ldata3b>
    c8fd:	41 ba 35 00 00 00    	mov    $0x35,%r10d
    c903:	48 0f b6 f8          	movzbq %al,%rdi
    c907:	4c 89 14 24          	mov    %r10,(%rsp)
    c90b:	48 8d 05 5e 64 00 00 	lea    0x645e(%rip),%rax        # 12d70 <pt_raise_msg>
    c912:	ff d0                	call   *%rax
    c914:	e9 49 fe ff ff       	jmp    c762 <py1_quicksort+0x422>
    c919:	48 8d 3d f8 25 04 00 	lea    0x425f8(%rip),%rdi        # 4ef18 <.Ldata39>
    c920:	be e9 00 00 00       	mov    $0xe9,%esi
    c925:	48 8d 05 44 5c 00 00 	lea    0x5c44(%rip),%rax        # 12570 <pt_fail>
    c92c:	ff d0                	call   *%rax
    c92e:	0f 0b                	ud2
    c930:	b9 01 00 00 00       	mov    $0x1,%ecx
    c935:	48 8d 35 39 24 04 00 	lea    0x42439(%rip),%rsi        # 4ed75 <.Ldata35>
    c93c:	ba 22 00 00 00       	mov    $0x22,%edx
    c941:	4c 8d 15 4f 24 04 00 	lea    0x4244f(%rip),%r10        # 4ed97 <.Ldata36>
    c948:	41 b8 5a 00 00 00    	mov    $0x5a,%r8d
    c94e:	4c 8d 0d 9c 24 04 00 	lea    0x4249c(%rip),%r9        # 4edf1 <.Ldata37>
    c955:	41 bb 39 00 00 00    	mov    $0x39,%r11d
    c95b:	48 0f b6 f9          	movzbq %cl,%rdi
    c95f:	4c 89 1c 24          	mov    %r11,(%rsp)
    c963:	4c 8d 1d 06 64 00 00 	lea    0x6406(%rip),%r11        # 12d70 <pt_raise_msg>
    c96a:	4c 89 d1             	mov    %r10,%rcx
    c96d:	41 ff d3             	call   *%r11
    c970:	e9 ed fd ff ff       	jmp    c762 <py1_quicksort+0x422>
    c975:	48 8d 3d 0c 23 04 00 	lea    0x4230c(%rip),%rdi        # 4ec88 <.Ldata34>
    c97c:	be ed 00 00 00       	mov    $0xed,%esi
    c981:	4c 8d 05 e8 5b 00 00 	lea    0x5be8(%rip),%r8        # 12570 <pt_fail>
    c988:	41 ff d0             	call   *%r8
    c98b:	0f 0b                	ud2
    c98d:	bf 01 00 00 00       	mov    $0x1,%edi
    c992:	48 8d 35 6d 0c 04 00 	lea    0x40c6d(%rip),%rsi        # 4d606 <.Ldatac>
    c999:	ba 17 00 00 00       	mov    $0x17,%edx
    c99e:	48 8d 0d 48 22 04 00 	lea    0x42248(%rip),%rcx        # 4ebed <.Ldata32>
    c9a5:	41 b8 5a 00 00 00    	mov    $0x5a,%r8d
    c9ab:	4c 8d 0d 95 22 04 00 	lea    0x42295(%rip),%r9        # 4ec47 <.Ldata33>
    c9b2:	41 ba 41 00 00 00    	mov    $0x41,%r10d
    c9b8:	48 0f b6 ff          	movzbq %dil,%rdi
    c9bc:	4c 89 14 24          	mov    %r10,(%rsp)
    c9c0:	4c 8d 15 a9 63 00 00 	lea    0x63a9(%rip),%r10        # 12d70 <pt_raise_msg>
    c9c7:	41 ff d2             	call   *%r10
    c9ca:	e9 93 fd ff ff       	jmp    c762 <py1_quicksort+0x422>
    c9cf:	48 8d 3d 22 21 04 00 	lea    0x42122(%rip),%rdi        # 4eaf8 <.Ldata31>
    c9d6:	be f5 00 00 00       	mov    $0xf5,%esi
    c9db:	4c 8d 1d 8e 5b 00 00 	lea    0x5b8e(%rip),%r11        # 12570 <pt_fail>
    c9e2:	41 ff d3             	call   *%r11
    c9e5:	0f 0b                	ud2
    c9e7:	b8 01 00 00 00       	mov    $0x1,%eax
    c9ec:	48 8d 35 13 0c 04 00 	lea    0x40c13(%rip),%rsi        # 4d606 <.Ldatac>
    c9f3:	ba 17 00 00 00       	mov    $0x17,%edx
    c9f8:	48 8d 0d 66 20 04 00 	lea    0x42066(%rip),%rcx        # 4ea65 <.Ldata2f>
    c9ff:	41 b8 5a 00 00 00    	mov    $0x5a,%r8d
    ca05:	4c 8d 0d b3 20 04 00 	lea    0x420b3(%rip),%r9        # 4eabf <.Ldata30>
    ca0c:	41 ba 39 00 00 00    	mov    $0x39,%r10d
    ca12:	48 0f b6 f8          	movzbq %al,%rdi
    ca16:	4c 89 14 24          	mov    %r10,(%rsp)
    ca1a:	48 8d 05 4f 63 00 00 	lea    0x634f(%rip),%rax        # 12d70 <pt_raise_msg>
    ca21:	ff d0                	call   *%rax
    ca23:	e9 3a fd ff ff       	jmp    c762 <py1_quicksort+0x422>
    ca28:	48 8d 3d 49 1f 04 00 	lea    0x41f49(%rip),%rdi        # 4e978 <.Ldata2e>
    ca2f:	be ed 00 00 00       	mov    $0xed,%esi
    ca34:	48 8d 05 35 5b 00 00 	lea    0x5b35(%rip),%rax        # 12570 <pt_fail>
    ca3b:	ff d0                	call   *%rax
    ca3d:	0f 0b                	ud2
    ca3f:	48 8d 3d 4b 1e 04 00 	lea    0x41e4b(%rip),%rdi        # 4e891 <.Ldata2d>
    ca46:	be e7 00 00 00       	mov    $0xe7,%esi
    ca4b:	48 8d 05 1e 5b 00 00 	lea    0x5b1e(%rip),%rax        # 12570 <pt_fail>
    ca52:	ff d0                	call   *%rax
    ca54:	0f 0b                	ud2
    ca56:	48 8d 3d 52 1d 04 00 	lea    0x41d52(%rip),%rdi        # 4e7af <.Ldata2c>
    ca5d:	be e2 00 00 00       	mov    $0xe2,%esi
    ca62:	48 8d 05 07 5b 00 00 	lea    0x5b07(%rip),%rax        # 12570 <pt_fail>
    ca69:	ff d0                	call   *%rax
    ca6b:	0f 0b                	ud2
    ca6d:	48 8d 3d 8f 1c 04 00 	lea    0x41c8f(%rip),%rdi        # 4e703 <.Ldata2b>
    ca74:	be ac 00 00 00       	mov    $0xac,%esi
    ca79:	48 8d 05 f0 5a 00 00 	lea    0x5af0(%rip),%rax        # 12570 <pt_fail>
    ca80:	ff d0                	call   *%rax
    ca82:	0f 0b                	ud2
    ca84:	48 8d 3d 95 1b 04 00 	lea    0x41b95(%rip),%rdi        # 4e620 <.Ldata2a>
    ca8b:	be e3 00 00 00       	mov    $0xe3,%esi
    ca90:	48 8d 05 d9 5a 00 00 	lea    0x5ad9(%rip),%rax        # 12570 <pt_fail>
    ca97:	ff d0                	call   *%rax
    ca99:	0f 0b                	ud2
    ca9b:	b8 01 00 00 00       	mov    $0x1,%eax
    caa0:	48 8d 35 5f 0b 04 00 	lea    0x40b5f(%rip),%rsi        # 4d606 <.Ldatac>
    caa7:	ba 17 00 00 00       	mov    $0x17,%edx
    caac:	48 8d 0d d6 19 04 00 	lea    0x419d6(%rip),%rcx        # 4e489 <.Ldata27>
    cab3:	41 b8 5a 00 00 00    	mov    $0x5a,%r8d
    cab9:	4c 8d 0d 23 1a 04 00 	lea    0x41a23(%rip),%r9        # 4e4e3 <.Ldata28>
    cac0:	41 ba 3e 00 00 00    	mov    $0x3e,%r10d
    cac6:	48 0f b6 f8          	movzbq %al,%rdi
    caca:	4c 89 14 24          	mov    %r10,(%rsp)
    cace:	48 8d 05 9b 62 00 00 	lea    0x629b(%rip),%rax        # 12d70 <pt_raise_msg>
    cad5:	ff d0                	call   *%rax
    cad7:	e9 86 fc ff ff       	jmp    c762 <py1_quicksort+0x422>
    cadc:	48 8d 3d b4 18 04 00 	lea    0x418b4(%rip),%rdi        # 4e397 <.Ldata26>
    cae3:	be f2 00 00 00       	mov    $0xf2,%esi
    cae8:	48 8d 05 81 5a 00 00 	lea    0x5a81(%rip),%rax        # 12570 <pt_fail>
    caef:	ff d0                	call   *%rax
    caf1:	0f 0b                	ud2
    caf3:	48 8d 3d f2 17 04 00 	lea    0x417f2(%rip),%rdi        # 4e2ec <.Ldata25>
    cafa:	be ab 00 00 00       	mov    $0xab,%esi
    caff:	48 8d 05 6a 5a 00 00 	lea    0x5a6a(%rip),%rax        # 12570 <pt_fail>
    cb06:	ff d0                	call   *%rax
    cb08:	0f 0b                	ud2
    cb0a:	48 8d 3d f9 16 04 00 	lea    0x416f9(%rip),%rdi        # 4e20a <.Ldata24>
    cb11:	be e2 00 00 00       	mov    $0xe2,%esi
    cb16:	48 8d 05 53 5a 00 00 	lea    0x5a53(%rip),%rax        # 12570 <pt_fail>
    cb1d:	ff d0                	call   *%rax
    cb1f:	0f 0b                	ud2
    cb21:	b8 01 00 00 00       	mov    $0x1,%eax
    cb26:	48 8d 35 d9 0a 04 00 	lea    0x40ad9(%rip),%rsi        # 4d606 <.Ldatac>
    cb2d:	ba 17 00 00 00       	mov    $0x17,%edx
    cb32:	48 8d 0d 3c 15 04 00 	lea    0x4153c(%rip),%rcx        # 4e075 <.Ldata21>
    cb39:	41 b8 59 00 00 00    	mov    $0x59,%r8d
    cb3f:	4c 8d 0d 88 15 04 00 	lea    0x41588(%rip),%r9        # 4e0ce <.Ldata22>
    cb46:	41 ba 3e 00 00 00    	mov    $0x3e,%r10d
    cb4c:	48 0f b6 f8          	movzbq %al,%rdi
    cb50:	4c 89 14 24          	mov    %r10,(%rsp)
    cb54:	48 8d 05 15 62 00 00 	lea    0x6215(%rip),%rax        # 12d70 <pt_raise_msg>
    cb5b:	ff d0                	call   *%rax
    cb5d:	e9 00 fc ff ff       	jmp    c762 <py1_quicksort+0x422>
    cb62:	48 8d 3d 1b 14 04 00 	lea    0x4141b(%rip),%rdi        # 4df84 <.Ldata20>
    cb69:	be f1 00 00 00       	mov    $0xf1,%esi
    cb6e:	48 8d 05 fb 59 00 00 	lea    0x59fb(%rip),%rax        # 12570 <pt_fail>
    cb75:	ff d0                	call   *%rax
    cb77:	0f 0b                	ud2
    cb79:	48 8d 3d 20 13 04 00 	lea    0x41320(%rip),%rdi        # 4dea0 <.Ldata1f>
    cb80:	be e4 00 00 00       	mov    $0xe4,%esi
    cb85:	48 8d 05 e4 59 00 00 	lea    0x59e4(%rip),%rax        # 12570 <pt_fail>
    cb8c:	ff d0                	call   *%rax
    cb8e:	0f 0b                	ud2
    cb90:	48 8d 3d 2a 12 04 00 	lea    0x4122a(%rip),%rdi        # 4ddc1 <.Ldata1e>
    cb97:	be df 00 00 00       	mov    $0xdf,%esi
    cb9c:	48 8d 05 cd 59 00 00 	lea    0x59cd(%rip),%rax        # 12570 <pt_fail>
    cba3:	ff d0                	call   *%rax
    cba5:	0f 0b                	ud2
    cba7:	b8 01 00 00 00       	mov    $0x1,%eax
    cbac:	48 8d 35 53 0a 04 00 	lea    0x40a53(%rip),%rsi        # 4d606 <.Ldatac>
    cbb3:	ba 17 00 00 00       	mov    $0x17,%edx
    cbb8:	48 8d 0d 5e 11 04 00 	lea    0x4115e(%rip),%rcx        # 4dd1d <.Ldata1c>
    cbbf:	41 b8 59 00 00 00    	mov    $0x59,%r8d
    cbc5:	4c 8d 0d aa 11 04 00 	lea    0x411aa(%rip),%r9        # 4dd76 <.Ldata1d>
    cbcc:	41 ba 4b 00 00 00    	mov    $0x4b,%r10d
    cbd2:	48 0f b6 f8          	movzbq %al,%rdi
    cbd6:	4c 89 14 24          	mov    %r10,(%rsp)
    cbda:	48 8d 05 8f 61 00 00 	lea    0x618f(%rip),%rax        # 12d70 <pt_raise_msg>
    cbe1:	ff d0                	call   *%rax
    cbe3:	e9 7a fb ff ff       	jmp    c762 <py1_quicksort+0x422>
    cbe8:	48 8d 3d 71 10 04 00 	lea    0x41071(%rip),%rdi        # 4dc60 <.Ldata1b>
    cbef:	be bd 00 00 00       	mov    $0xbd,%esi
    cbf4:	48 8d 05 75 59 00 00 	lea    0x5975(%rip),%rax        # 12570 <pt_fail>
    cbfb:	ff d0                	call   *%rax
    cbfd:	0f 0b                	ud2

Disassembly of section .fini:

/workspace/scratch/2ce9dd46a989/card814-candidate-final/sorting:     file format elf64-x86-64


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
    bd56:	4c 8d 05 73 44 00 00 	lea    0x4473(%rip),%r8        # 101d0 <pt_seq_new>
    bd5d:	48 89 da             	mov    %rbx,%rdx
    bd60:	41 ff d0             	call   *%r8
    bd63:	48 89 44 24 48       	mov    %rax,0x48(%rsp)
    bd68:	b8 39 30 00 00       	mov    $0x3039,%eax
    bd6d:	48 81 fb e0 93 04 00 	cmp    $0x493e0,%rbx
    bd74:	0f 8d e2 00 00 00    	jge    be5c <pythrust_main+0x13c>
    bd7a:	48 f7 2d 9f 05 00 00 	imulq  0x59f(%rip)        # c320 <pythrust_main+0x600>
    bd81:	0f 80 7e 05 00 00    	jo     c305 <pythrust_main+0x5e5>
    bd87:	48 05 39 30 00 00    	add    $0x3039,%rax
    bd8d:	41 0f 90 c0          	seto   %r8b
    bd91:	45 84 c0             	test   %r8b,%r8b
    bd94:	0f 85 54 05 00 00    	jne    c2ee <pythrust_main+0x5ce>
    bd9a:	49 89 c2             	mov    %rax,%r10
    bd9d:	49 c1 fa 1e          	sar    $0x1e,%r10
    bda1:	49 c1 ea 21          	shr    $0x21,%r10
    bda5:	4c 03 d0             	add    %rax,%r10
    bda8:	49 81 e2 00 00 00 80 	and    $0xffffffff80000000,%r10
    bdaf:	49 89 c3             	mov    %rax,%r11
    bdb2:	4d 2b da             	sub    %r10,%r11
    bdb5:	49 3b c2             	cmp    %r10,%rax
    bdb8:	0f 95 c0             	setne  %al
    bdbb:	4d 89 d9             	mov    %r11,%r9
    bdbe:	4c 33 0d 63 05 00 00 	xor    0x563(%rip),%r9        # c328 <pythrust_main+0x608>
    bdc5:	48 33 c9             	xor    %rcx,%rcx
    bdc8:	49 c1 e9 3f          	shr    $0x3f,%r9
    bdcc:	41 84 c1             	test   %al,%r9b
    bdcf:	48 89 c8             	mov    %rcx,%rax
    bdd2:	48 0f 45 05 4e 05 00 	cmovne 0x54e(%rip),%rax        # c328 <pythrust_main+0x608>
    bdd9:	00 
    bdda:	49 03 c3             	add    %r11,%rax
    bddd:	49 89 c4             	mov    %rax,%r12
    bde0:	48 f7 2d 49 05 00 00 	imulq  0x549(%rip)        # c330 <pythrust_main+0x610>
    bde7:	48 c1 fa 12          	sar    $0x12,%rdx
    bdeb:	48 89 d0             	mov    %rdx,%rax
    bdee:	48 c1 e8 3f          	shr    $0x3f,%rax
    bdf2:	48 03 c2             	add    %rdx,%rax
    bdf5:	48 69 d0 40 42 0f 00 	imul   $0xf4240,%rax,%rdx
    bdfc:	4c 89 e0             	mov    %r12,%rax
    bdff:	48 2b c2             	sub    %rdx,%rax
    be02:	4c 3b e2             	cmp    %rdx,%r12
    be05:	0f 95 c2             	setne  %dl
    be08:	48 89 c6             	mov    %rax,%rsi
    be0b:	48 81 f6 40 42 0f 00 	xor    $0xf4240,%rsi
    be12:	48 c1 ee 3f          	shr    $0x3f,%rsi
    be16:	40 84 d6             	test   %dl,%sil
    be19:	48 0f 45 0d 17 05 00 	cmovne 0x517(%rip),%rcx        # c338 <pythrust_main+0x618>
    be20:	00 
    be21:	48 8d 34 08          	lea    (%rax,%rcx,1),%rsi
    be25:	48 8d 05 74 44 00 00 	lea    0x4474(%rip),%rax        # 102a0 <pt_seq_push>
    be2c:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    be31:	ff d0                	call   *%rax
    be33:	48 89 d8             	mov    %rbx,%rax
    be36:	48 83 c0 01          	add    $0x1,%rax
    be3a:	0f 90 c0             	seto   %al
    be3d:	84 c0                	test   %al,%al
    be3f:	0f 85 17 00 00 00    	jne    be5c <pythrust_main+0x13c>
    be45:	48 83 c3 01          	add    $0x1,%rbx
    be49:	0f 90 c0             	seto   %al
    be4c:	84 c0                	test   %al,%al
    be4e:	0f 85 9b 02 00 00    	jne    c0ef <pythrust_main+0x3cf>
    be54:	4c 89 e0             	mov    %r12,%rax
    be57:	e9 11 ff ff ff       	jmp    bd6d <pythrust_main+0x4d>
    be5c:	48 8d 05 ed 42 00 00 	lea    0x42ed(%rip),%rax        # 10150 <pt_seq_len>
    be63:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    be68:	ff d0                	call   *%rax
    be6a:	48 89 c1             	mov    %rax,%rcx
    be6d:	48 83 e9 01          	sub    $0x1,%rcx
    be71:	0f 90 c0             	seto   %al
    be74:	84 c0                	test   %al,%al
    be76:	0f 85 5b 04 00 00    	jne    c2d7 <pythrust_main+0x5b7>
    be7c:	4d 89 ef             	mov    %r13,%r15
    be7f:	49 8b 5f 08          	mov    0x8(%r15),%rbx
    be83:	48 81 fb e8 03 00 00 	cmp    $0x3e8,%rbx
    be8a:	0f 8d 30 04 00 00    	jge    c2c0 <pythrust_main+0x5a0>
    be90:	48 8d 43 01          	lea    0x1(%rbx),%rax
    be94:	49 89 47 08          	mov    %rax,0x8(%r15)
    be98:	48 33 d2             	xor    %rdx,%rdx
    be9b:	48 8b 74 24 48       	mov    0x48(%rsp),%rsi
    bea0:	4c 89 ff             	mov    %r15,%rdi
    bea3:	e8 98 04 00 00       	call   c340 <py1_quicksort>
    bea8:	49 89 5f 08          	mov    %rbx,0x8(%r15)
    beac:	49 0f b6 07          	movzbq (%r15),%rax
    beb0:	84 c0                	test   %al,%al
    beb2:	0f 85 ef 03 00 00    	jne    c2a7 <pythrust_main+0x587>
    beb8:	48 8d 05 91 42 00 00 	lea    0x4291(%rip),%rax        # 10150 <pt_seq_len>
    bebf:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bec4:	ff d0                	call   *%rax
    bec6:	48 89 44 24 40       	mov    %rax,0x40(%rsp)
    becb:	41 bc 01 00 00 00    	mov    $0x1,%r12d
    bed1:	bb 01 00 00 00       	mov    $0x1,%ebx
    bed6:	4c 8d 6c 24 10       	lea    0x10(%rsp),%r13
    bedb:	4c 8d 74 24 18       	lea    0x18(%rsp),%r14
    bee0:	48 8b 4c 24 40       	mov    0x40(%rsp),%rcx
    bee5:	49 89 df             	mov    %rbx,%r15
    bee8:	4c 3b e1             	cmp    %rcx,%r12
    beeb:	48 89 4c 24 40       	mov    %rcx,0x40(%rsp)
    bef0:	0f 8d 9b 00 00 00    	jge    bf91 <pythrust_main+0x271>
    bef6:	4c 89 e6             	mov    %r12,%rsi
    bef9:	48 83 ee 01          	sub    $0x1,%rsi
    befd:	0f 90 c0             	seto   %al
    bf00:	84 c0                	test   %al,%al
    bf02:	0f 85 88 03 00 00    	jne    c290 <pythrust_main+0x570>
    bf08:	48 8d 05 a1 41 00 00 	lea    0x41a1(%rip),%rax        # 100b0 <pt_seq_get>
    bf0f:	4c 89 ea             	mov    %r13,%rdx
    bf12:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bf17:	ff d0                	call   *%rax
    bf19:	48 89 44 24 38       	mov    %rax,0x38(%rsp)
    bf1e:	48 0f b6 44 24 10    	movzbq 0x10(%rsp),%rax
    bf24:	3c 01                	cmp    $0x1,%al
    bf26:	0f 84 23 03 00 00    	je     c24f <pythrust_main+0x52f>
    bf2c:	48 8d 05 7d 41 00 00 	lea    0x417d(%rip),%rax        # 100b0 <pt_seq_get>
    bf33:	4c 89 f2             	mov    %r14,%rdx
    bf36:	4c 89 e6             	mov    %r12,%rsi
    bf39:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bf3e:	ff d0                	call   *%rax
    bf40:	48 0f b6 4c 24 18    	movzbq 0x18(%rsp),%rcx
    bf46:	40 80 f9 01          	rex cmp $0x1,%cl
    bf4a:	0f 84 be 02 00 00    	je     c20e <pythrust_main+0x4ee>
    bf50:	4c 8b 5c 24 38       	mov    0x38(%rsp),%r11
    bf55:	4c 3b d8             	cmp    %rax,%r11
    bf58:	0f 8e 08 00 00 00    	jle    bf66 <pythrust_main+0x246>
    bf5e:	bb 01 00 00 00       	mov    $0x1,%ebx
    bf63:	45 33 ff             	xor    %r15d,%r15d
    bf66:	4c 89 e0             	mov    %r12,%rax
    bf69:	48 83 c0 01          	add    $0x1,%rax
    bf6d:	0f 90 c0             	seto   %al
    bf70:	84 c0                	test   %al,%al
    bf72:	0f 85 19 00 00 00    	jne    bf91 <pythrust_main+0x271>
    bf78:	49 83 c4 01          	add    $0x1,%r12
    bf7c:	0f 90 c0             	seto   %al
    bf7f:	84 c0                	test   %al,%al
    bf81:	0f 85 7f 01 00 00    	jne    c106 <pythrust_main+0x3e6>
    bf87:	48 8b 4c 24 40       	mov    0x40(%rsp),%rcx
    bf8c:	e9 57 ff ff ff       	jmp    bee8 <pythrust_main+0x1c8>
    bf91:	84 db                	test   %bl,%bl
    bf93:	0f 84 5e 02 00 00    	je     c1f7 <pythrust_main+0x4d7>
    bf99:	48 33 f6             	xor    %rsi,%rsi
    bf9c:	48 8d 54 24 20       	lea    0x20(%rsp),%rdx
    bfa1:	48 8d 0d 08 41 00 00 	lea    0x4108(%rip),%rcx        # 100b0 <pt_seq_get>
    bfa8:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bfad:	ff d1                	call   *%rcx
    bfaf:	49 89 c6             	mov    %rax,%r14
    bfb2:	48 0f b6 4c 24 20    	movzbq 0x20(%rsp),%rcx
    bfb8:	40 80 f9 01          	rex cmp $0x1,%cl
    bfbc:	0f 84 f4 01 00 00    	je     c1b6 <pythrust_main+0x496>
    bfc2:	48 8d 35 87 41 00 00 	lea    0x4187(%rip),%rsi        # 10150 <pt_seq_len>
    bfc9:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bfce:	ff d6                	call   *%rsi
    bfd0:	48 89 c6             	mov    %rax,%rsi
    bfd3:	48 d1 fe             	sar    $1,%rsi
    bfd6:	48 8d 54 24 28       	lea    0x28(%rsp),%rdx
    bfdb:	4c 8d 05 ce 40 00 00 	lea    0x40ce(%rip),%r8        # 100b0 <pt_seq_get>
    bfe2:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    bfe7:	41 ff d0             	call   *%r8
    bfea:	48 89 c3             	mov    %rax,%rbx
    bfed:	48 0f b6 74 24 28    	movzbq 0x28(%rsp),%rsi
    bff3:	40 80 fe 01          	cmp    $0x1,%sil
    bff7:	0f 84 78 01 00 00    	je     c175 <pythrust_main+0x455>
    bffd:	4c 8d 05 4c 41 00 00 	lea    0x414c(%rip),%r8        # 10150 <pt_seq_len>
    c004:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    c009:	41 ff d0             	call   *%r8
    c00c:	48 89 c6             	mov    %rax,%rsi
    c00f:	48 83 ee 01          	sub    $0x1,%rsi
    c013:	41 0f 90 c0          	seto   %r8b
    c017:	45 84 c0             	test   %r8b,%r8b
    c01a:	0f 85 3e 01 00 00    	jne    c15e <pythrust_main+0x43e>
    c020:	48 8d 54 24 30       	lea    0x30(%rsp),%rdx
    c025:	4c 8d 0d 84 40 00 00 	lea    0x4084(%rip),%r9        # 100b0 <pt_seq_get>
    c02c:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    c031:	41 ff d1             	call   *%r9
    c034:	49 89 c4             	mov    %rax,%r12
    c037:	4c 0f b6 4c 24 30    	movzbq 0x30(%rsp),%r9
    c03d:	41 80 f9 01          	cmp    $0x1,%r9b
    c041:	0f 84 d6 00 00 00    	je     c11d <pythrust_main+0x3fd>
    c047:	49 0f b6 ff          	movzbq %r15b,%rdi
    c04b:	48 8d 05 9e 65 00 00 	lea    0x659e(%rip),%rax        # 125f0 <pt_write_bool>
    c052:	ff d0                	call   *%rax
    c054:	41 bd 20 00 00 00    	mov    $0x20,%r13d
    c05a:	49 0f b6 fd          	movzbq %r13b,%rdi
    c05e:	48 8d 05 cb 65 00 00 	lea    0x65cb(%rip),%rax        # 12630 <pt_write_byte>
    c065:	ff d0                	call   *%rax
    c067:	48 8d 05 f2 65 00 00 	lea    0x65f2(%rip),%rax        # 12660 <pt_write_int>
    c06e:	4c 89 f7             	mov    %r14,%rdi
    c071:	ff d0                	call   *%rax
    c073:	49 0f b6 fd          	movzbq %r13b,%rdi
    c077:	48 8d 05 b2 65 00 00 	lea    0x65b2(%rip),%rax        # 12630 <pt_write_byte>
    c07e:	ff d0                	call   *%rax
    c080:	48 8d 05 d9 65 00 00 	lea    0x65d9(%rip),%rax        # 12660 <pt_write_int>
    c087:	48 89 df             	mov    %rbx,%rdi
    c08a:	ff d0                	call   *%rax
    c08c:	49 0f b6 fd          	movzbq %r13b,%rdi
    c090:	48 8d 05 99 65 00 00 	lea    0x6599(%rip),%rax        # 12630 <pt_write_byte>
    c097:	ff d0                	call   *%rax
    c099:	48 8d 05 c0 65 00 00 	lea    0x65c0(%rip),%rax        # 12660 <pt_write_int>
    c0a0:	4c 89 e7             	mov    %r12,%rdi
    c0a3:	ff d0                	call   *%rax
    c0a5:	b8 0a 00 00 00       	mov    $0xa,%eax
    c0aa:	48 0f b6 f8          	movzbq %al,%rdi
    c0ae:	48 8d 05 7b 65 00 00 	lea    0x657b(%rip),%rax        # 12630 <pt_write_byte>
    c0b5:	ff d0                	call   *%rax
    c0b7:	48 8d 05 12 3f 00 00 	lea    0x3f12(%rip),%rax        # ffd0 <pt_seq_dec>
    c0be:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    c0c3:	ff d0                	call   *%rax
    c0c5:	48 8b 5c 24 50       	mov    0x50(%rsp),%rbx
    c0ca:	4c 8b 64 24 58       	mov    0x58(%rsp),%r12
    c0cf:	4c 8b 6c 24 60       	mov    0x60(%rsp),%r13
    c0d4:	4c 8b 74 24 68       	mov    0x68(%rsp),%r14
    c0d9:	4c 8b 7c 24 70       	mov    0x70(%rsp),%r15
    c0de:	48 81 c4 80 00 00 00 	add    $0x80,%rsp
    c0e5:	48 89 ec             	mov    %rbp,%rsp
    c0e8:	5d                   	pop    %rbp
    c0e9:	c3                   	ret
    c0ea:	e9 b8 01 00 00       	jmp    c2a7 <pythrust_main+0x587>
    c0ef:	48 8d 3d 38 12 04 00 	lea    0x41238(%rip),%rdi        # 4d32e <.Ldata8>
    c0f6:	be ad 00 00 00       	mov    $0xad,%esi
    c0fb:	48 8d 05 6e 64 00 00 	lea    0x646e(%rip),%rax        # 12570 <pt_fail>
    c102:	ff d0                	call   *%rax
    c104:	0f 0b                	ud2
    c106:	48 8d 3d 99 15 04 00 	lea    0x41599(%rip),%rdi        # 4d6a6 <.Ldataf>
    c10d:	be b5 00 00 00       	mov    $0xb5,%esi
    c112:	48 8d 05 57 64 00 00 	lea    0x6457(%rip),%rax        # 12570 <pt_fail>
    c119:	ff d0                	call   *%rax
    c11b:	0f 0b                	ud2
    c11d:	b8 01 00 00 00       	mov    $0x1,%eax
    c122:	48 8d 35 dd 14 04 00 	lea    0x414dd(%rip),%rsi        # 4d606 <.Ldatac>
    c129:	ba 17 00 00 00       	mov    $0x17,%edx
    c12e:	48 8d 0d 65 1a 04 00 	lea    0x41a65(%rip),%rcx        # 4db9a <.Ldata19>
    c135:	41 b8 5a 00 00 00    	mov    $0x5a,%r8d
    c13b:	4c 8d 0d b2 1a 04 00 	lea    0x41ab2(%rip),%r9        # 4dbf4 <.Ldata1a>
    c142:	41 ba 6c 00 00 00    	mov    $0x6c,%r10d
    c148:	48 0f b6 f8          	movzbq %al,%rdi
    c14c:	4c 89 14 24          	mov    %r10,(%rsp)
    c150:	48 8d 05 19 6c 00 00 	lea    0x6c19(%rip),%rax        # 12d70 <pt_raise_msg>
    c157:	ff d0                	call   *%rax
    c159:	e9 49 01 00 00       	jmp    c2a7 <pythrust_main+0x587>
    c15e:	48 8d 3d 51 19 04 00 	lea    0x41951(%rip),%rdi        # 4dab6 <.Ldata18>
    c165:	be e4 00 00 00       	mov    $0xe4,%esi
    c16a:	48 8d 05 ff 63 00 00 	lea    0x63ff(%rip),%rax        # 12570 <pt_fail>
    c171:	ff d0                	call   *%rax
    c173:	0f 0b                	ud2
    c175:	b8 01 00 00 00       	mov    $0x1,%eax
    c17a:	48 8d 35 85 14 04 00 	lea    0x41485(%rip),%rsi        # 4d606 <.Ldatac>
    c181:	ba 17 00 00 00       	mov    $0x17,%edx
    c186:	48 8d 0d 74 18 04 00 	lea    0x41874(%rip),%rcx        # 4da01 <.Ldata16>
    c18d:	41 b8 5a 00 00 00    	mov    $0x5a,%r8d
    c193:	4c 8d 0d c1 18 04 00 	lea    0x418c1(%rip),%r9        # 4da5b <.Ldata17>
    c19a:	41 ba 5b 00 00 00    	mov    $0x5b,%r10d
    c1a0:	48 0f b6 f8          	movzbq %al,%rdi
    c1a4:	4c 89 14 24          	mov    %r10,(%rsp)
    c1a8:	48 8d 05 c1 6b 00 00 	lea    0x6bc1(%rip),%rax        # 12d70 <pt_raise_msg>
    c1af:	ff d0                	call   *%rax
    c1b1:	e9 f1 00 00 00       	jmp    c2a7 <pythrust_main+0x587>
    c1b6:	b8 01 00 00 00       	mov    $0x1,%eax
    c1bb:	48 8d 35 44 14 04 00 	lea    0x41444(%rip),%rsi        # 4d606 <.Ldatac>
    c1c2:	ba 17 00 00 00       	mov    $0x17,%edx
    c1c7:	48 8d 0d 90 17 04 00 	lea    0x41790(%rip),%rcx        # 4d95e <.Ldata14>
    c1ce:	41 b8 5a 00 00 00    	mov    $0x5a,%r8d
    c1d4:	4c 8d 0d dd 17 04 00 	lea    0x417dd(%rip),%r9        # 4d9b8 <.Ldata15>
    c1db:	41 ba 49 00 00 00    	mov    $0x49,%r10d
    c1e1:	48 0f b6 f8          	movzbq %al,%rdi
    c1e5:	4c 89 14 24          	mov    %r10,(%rsp)
    c1e9:	48 8d 05 80 6b 00 00 	lea    0x6b80(%rip),%rax        # 12d70 <pt_raise_msg>
    c1f0:	ff d0                	call   *%rax
    c1f2:	e9 b0 00 00 00       	jmp    c2a7 <pythrust_main+0x587>
    c1f7:	48 8d 3d a2 16 04 00 	lea    0x416a2(%rip),%rdi        # 4d8a0 <.Ldata13>
    c1fe:	be be 00 00 00       	mov    $0xbe,%esi
    c203:	48 8d 05 66 63 00 00 	lea    0x6366(%rip),%rax        # 12570 <pt_fail>
    c20a:	ff d0                	call   *%rax
    c20c:	0f 0b                	ud2
    c20e:	b8 01 00 00 00       	mov    $0x1,%eax
    c213:	48 8d 35 ec 13 04 00 	lea    0x413ec(%rip),%rsi        # 4d606 <.Ldatac>
    c21a:	ba 17 00 00 00       	mov    $0x17,%edx
    c21f:	48 8d 0d e8 15 04 00 	lea    0x415e8(%rip),%rcx        # 4d80e <.Ldata11>
    c226:	41 b8 5a 00 00 00    	mov    $0x5a,%r8d
    c22c:	4c 8d 0d 35 16 04 00 	lea    0x41635(%rip),%r9        # 4d868 <.Ldata12>
    c233:	41 ba 38 00 00 00    	mov    $0x38,%r10d
    c239:	48 0f b6 f8          	movzbq %al,%rdi
    c23d:	4c 89 14 24          	mov    %r10,(%rsp)
    c241:	48 8d 05 28 6b 00 00 	lea    0x6b28(%rip),%rax        # 12d70 <pt_raise_msg>
    c248:	ff d0                	call   *%rax
    c24a:	e9 58 00 00 00       	jmp    c2a7 <pythrust_main+0x587>
    c24f:	b8 01 00 00 00       	mov    $0x1,%eax
    c254:	48 8d 35 ab 13 04 00 	lea    0x413ab(%rip),%rsi        # 4d606 <.Ldatac>
    c25b:	ba 17 00 00 00       	mov    $0x17,%edx
    c260:	48 8d 0d b6 13 04 00 	lea    0x413b6(%rip),%rcx        # 4d61d <.Ldatad>
    c267:	41 b8 59 00 00 00    	mov    $0x59,%r8d
    c26d:	4c 8d 0d 02 14 04 00 	lea    0x41402(%rip),%r9        # 4d676 <.Ldatae>
    c274:	41 ba 30 00 00 00    	mov    $0x30,%r10d
    c27a:	48 0f b6 f8          	movzbq %al,%rdi
    c27e:	4c 89 14 24          	mov    %r10,(%rsp)
    c282:	48 8d 05 e7 6a 00 00 	lea    0x6ae7(%rip),%rax        # 12d70 <pt_raise_msg>
    c289:	ff d0                	call   *%rax
    c28b:	e9 17 00 00 00       	jmp    c2a7 <pythrust_main+0x587>
    c290:	48 8d 3d c7 12 04 00 	lea    0x412c7(%rip),%rdi        # 4d55e <.Ldatab>
    c297:	be a8 00 00 00       	mov    $0xa8,%esi
    c29c:	48 8d 05 cd 62 00 00 	lea    0x62cd(%rip),%rax        # 12570 <pt_fail>
    c2a3:	ff d0                	call   *%rax
    c2a5:	0f 0b                	ud2
    c2a7:	48 8d 05 22 3d 00 00 	lea    0x3d22(%rip),%rax        # ffd0 <pt_seq_dec>
    c2ae:	48 8b 7c 24 48       	mov    0x48(%rsp),%rdi
    c2b3:	ff d0                	call   *%rax
    c2b5:	48 8d 05 a4 6b 00 00 	lea    0x6ba4(%rip),%rax        # 12e60 <pt_uncaught>
    c2bc:	ff d0                	call   *%rax
    c2be:	0f 0b                	ud2
    c2c0:	48 8d 3d cd 11 04 00 	lea    0x411cd(%rip),%rdi        # 4d494 <.Ldataa>
    c2c7:	be ca 00 00 00       	mov    $0xca,%esi
    c2cc:	48 8d 05 9d 62 00 00 	lea    0x629d(%rip),%rax        # 12570 <pt_fail>
    c2d3:	ff d0                	call   *%rax
    c2d5:	0f 0b                	ud2
    c2d7:	48 8d 3d fd 10 04 00 	lea    0x410fd(%rip),%rdi        # 4d3db <.Ldata9>
    c2de:	be b9 00 00 00       	mov    $0xb9,%esi
    c2e3:	48 8d 05 86 62 00 00 	lea    0x6286(%rip),%rax        # 12570 <pt_fail>
    c2ea:	ff d0                	call   *%rax
    c2ec:	0f 0b                	ud2
    c2ee:	48 8d 3d dd 0d 04 00 	lea    0x40ddd(%rip),%rdi        # 4d0d2 <.Ldata2>
    c2f5:	be d9 00 00 00       	mov    $0xd9,%esi
    c2fa:	48 8d 05 6f 62 00 00 	lea    0x626f(%rip),%rax        # 12570 <pt_fail>
    c301:	ff d0                	call   *%rax
    c303:	0f 0b                	ud2
    c305:	48 8d 3d f6 0c 04 00 	lea    0x40cf6(%rip),%rdi        # 4d002 <.Ldata1>
    c30c:	be d0 00 00 00       	mov    $0xd0,%esi
    c311:	48 8d 05 58 62 00 00 	lea    0x6258(%rip),%rax        # 12570 <pt_fail>
    c318:	ff d0                	call   *%rax
    c31a:	0f 0b                	ud2
    c31c:	00 00                	add    %al,(%rax)
    c31e:	00 00                	add    %al,(%rax)
    c320:	6d                   	insl   (%dx),%es:(%rdi)
    c321:	4e c6 41 00 00       	rex.WRX movb $0x0,0x0(%rcx)
    c326:	00 00                	add    %al,(%rax)
    c328:	00 00                	add    %al,(%rax)
    c32a:	00 80 00 00 00 00    	add    %al,0x0(%rax)
    c330:	db 34 b6             	(bad) (%rsi,%rsi,4)
    c333:	d7                   	xlat   %ds:(%rbx)
    c334:	82                   	(bad)
    c335:	de 1b                	ficomps (%rbx)
    c337:	43                   	rex.XB
    c338:	40                   	rex
    c339:	42 0f 00 00          	rex.X sldt (%rax)
    c33d:	00 00                	add    %al,(%rax)
	...

Disassembly of section .fini:
