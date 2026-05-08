from starisc import private, public, claim, const

EXP_BITS = const("EXP_BITS")

encrypted = public(0)
n = public(1)

res = 1
for i in range(EXP_BITS):
    b = private(i)
    sq = (res * res) % n
    if b:
        res = (sq * encrypted) % n
    else:
        res = sq

claim(res)
