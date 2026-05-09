from starisc import private, public, claim

x = private(0)
lower = public(0)
upper = public(1)

gt_lower = x > lower
lt_upper = x < upper
in_range = gt_lower + lt_upper
claim(in_range)
