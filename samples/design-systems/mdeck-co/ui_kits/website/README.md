# MDeck Co: Website UI kit

The MDeck Co consulting firm's public site. A recreation composed entirely
from the design-system primitives (`Button`, `Card`, `Input`, etc.).

## Screens / flow
`index.html` is an interactive click-through:

1. **Home**: hero over the mood photograph ("Bring us the hard
   problems."), selected-work grid, the ethos band with stat blocks, and a
   closing access band.
2. **Request access**: click any *Bring us your challenge* button to enter
   the enquiry flow: a "black card" left rail + a form on the right.
3. **Confirmation**: submitting the form shows the "We've got it" state.

## Files
- `Chrome.jsx`: `Nav` (sticky, blurred) + `Footer` (with the signature line).
- `Home.jsx`: `Hero`, `Work`, `Ethos`, `AccessBand`.
- `Apply.jsx`: the `Apply` access flow + confirmation.
- `index.html`: routes between home and apply.

## Notes
- Icons are Lucide (loaded from CDN).
- Everything reads brand tokens from `../../styles.css`; the dark theme
  is the only theme.
