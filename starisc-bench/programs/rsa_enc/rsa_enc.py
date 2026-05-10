from starisc import private, public, claim

message = private(0)
n = public(0)

e = 65537

encrypted = (message**e) % n

claim(encrypted)
