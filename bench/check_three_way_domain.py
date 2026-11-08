"""Independent untimed dataset/domain oracle for #813; memoization only here."""
import hashlib
import json


def check():
    cache={1:0}; peak=1; total=0; longest=0
    for i in range(1,1000000):
        n=i; chain=[]
        while n not in cache:
            chain.append(n)
            n=n//2 if n%2==0 else 3*n+1
            peak=max(peak,n)
            if not 0<n<2**63: raise ValueError('Collatz outside i64')
        count=cache[n]
        for n in reversed(chain):
            count+=1; cache[n]=count
        total+=cache[i]; longest=max(longest,cache[i])
    if (total,longest)!=(131434272,524): raise ValueError('Collatz oracle disagreement')
    seed=12345; xs=[]; max_product=0
    for _ in range(300000):
        product=seed*1103515245; max_product=max(max_product,product)
        if product+12345>=2**63: raise ValueError('LCG outside i64')
        seed=(product+12345)%2147483648; xs.append(seed%1000000)
    encode=lambda xs: ''.join(str(n)+'\n' for n in xs).encode()
    input_hash=hashlib.sha256(encode(xs)).hexdigest()
    xs.sort()
    if (xs[0],xs[len(xs)//2],xs[-1])!=(6,499909,999999): raise ValueError('sort oracle disagreement')
    return dict(kind='untimed independent oracle; no performance data',
                collatz=dict(total=total,longest=longest,maximum_value=peak,cache_entries=len(cache)),
                sorting=dict(count=len(xs),maximum_lcg_product=max_product,input_sha256=input_hash,
                             sorted_sha256=hashlib.sha256(encode(xs)).hexdigest(),encoding='ASCII decimal integer plus LF per element'),
                integer_domain='All timed positive arithmetic within signed i64; checked Rust full runs additionally validate executed arithmetic')


if __name__=='__main__': print(json.dumps(check(),indent=2))
