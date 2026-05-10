from starisc import private, claim, const

N = const("N")

a = private(0)
b = private(1)
for i in range(N):
    c = a + b
    a = b
    b = c
claim(a)
