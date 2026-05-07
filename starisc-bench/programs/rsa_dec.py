from starisc import private, public, claim

encrypted = public(0)
n = public(1)

res = 1
for i in range(32):  # d as 32 private bits, square-and-multiply
    b = private(i)
    sq = (res * res) % n
    if b:
        res = (sq * encrypted) % n
    else:
        res = sq

claim(res)
