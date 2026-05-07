from starisc import private, claim

a = private(0)
b = private(1)
for i in range(4):
    a = a + b
    b = a + b
claim(a)
