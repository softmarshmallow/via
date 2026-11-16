def generate(step):
    # A mast at mid-river on the terrain, scaled 3 so its top (25 m) reaches above the deck
    # (24.5 m), as the `through` case's hand mast; and a box far from the river.
    step.out.instance("mast", "box", at=[142000, 168000], scale=3)
    step.out.instance("spare", "box", at=[20000, 20000])
