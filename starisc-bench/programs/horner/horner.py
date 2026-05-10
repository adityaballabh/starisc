from starisc import private, claim, const

N = const("N")

x = private(0)
acc = 0
for i in range(N):
    coeff = i + 1
    acc = acc * x + coeff
claim(acc)
