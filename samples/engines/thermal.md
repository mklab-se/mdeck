---
title: What your eyes can't see
author: Field Applications
theme: thermal
---

# What your eyes can't see

Thermal imaging in the field

```@notes
The cold opening: the title forms in heat before the type settles. The
pictures in this deck are synthetic examples, made for the MDeck samples.
```

# An ordinary cabinet

```@thermal
image: ../images/thermal/cabinet.jpg
visible: ../images/thermal/cabinet-visible.jpg
label: Cabinet 4, breaker row B (synthetic example images)
+ lens 76% 43% 16%
+ reveal
+ spot Hotspot 76% 43%
+ spot Reference 30% 52%
```

```@notes
Click 1: the lens finds something. Click 2: the whole picture. Clicks 3
and 4 mark the hotspot and the reference. The next
slide zooms into the hotspot.
```

# The hotspot, up close
<!-- zoom-to: Hotspot -->

```@thermal
image: ../images/thermal/cabinet-closeup.jpg
label: Breaker B3, terminal 2 (synthetic example image)
+ above 85%
+ above 65%
+ above 45%
```

```@notes
Each click colours a wider band, from the hottest metal outward.
```

# The method

# Seeing heat
<!-- picture: thermographer -->

- Every surface gives off infrared radiation
+ Hotter surfaces give off more of it
+ The camera turns that radiation into a picture
+ A palette turns the picture into heat you can read

# Where the heat goes

```@bar
# y-label: Temperature rise (K)
- Breaker B3: 55
- Breaker B2: 12
- Breaker B1: 9
- Cabinet air: 6
```

# Before and after the repair
<!--
design: columns
thermal-window: 25..90 °C
-->

```@thermal
image: ../images/thermal/cabinet-before.png
mapping: linear 18..92 °C
label: Before (synthetic example)
- spot B3 76% 43%
```

+++

```@thermal
data: ../images/thermal/cabinet-after.thermal.png
label: After (synthetic example)
- spot B3 76% 43%
```

```@notes
One common scale, so the colours mean the same on both sides. The
before image is a linear export (values are approximate, ≈); the after
image is temperature data with a sidecar.
```
