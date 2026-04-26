p = 70001
q = 60013

n = p * q
phi = (p - 1) * (q - 1)

e = 65537
d = 2145513473

message = 1337
encrypted = (message**e) % n
decrypted = (encrypted**d) % n

assert message == decrypted
