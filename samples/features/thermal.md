---
title: Thermal images
@theme: dark
---

# Thermal images

Every `@thermal` feature on one deck (all images are synthetic examples)

# A thermal image, as is

```@thermal
image: ../images/thermal/house.jpg
label: A grayscale white-hot export, coloured with the iron palette
```

# Any palette

```@thermal
image: ../images/thermal/solar.jpg
palette: arctic
label: palette: arctic (press C while presenting to try the others)
```

# A lens over the photo

```@thermal
image: ../images/thermal/cabinet.jpg
visible: ../images/thermal/cabinet-visible.jpg
+ lens 30% 60% 14%
+ lens 76% 43% 16%
+ reveal
* spot Hotspot 76% 43%
```

# A lens without a photo

```@thermal
image: ../images/thermal/circuit.jpg
label: Without visible:, the image waits in gray under the lens
+ lens 32% 42% 16%
+ reveal
```

# Threshold reveal

```@thermal
image: ../images/thermal/circuit.jpg
palette: lava
+ above 85%
+ above 70%
+ above 55%
```

# Author-supplied values

```@thermal
image: ../images/thermal/cabinet.jpg
- spot Sp1 76% 43%: about 86 °C
- spot Sp2 30% 52%
```

# A colour export

```@thermal
image: ../images/thermal/cabinet-iron.jpg
visible: ../images/thermal/cabinet-visible.jpg
+ lens 76% 43% 16%
+ reveal
```

# A linear mapping

```@thermal
image: ../images/thermal/cabinet-before.png
mapping: linear 18..92 °C
window: 30..90 °C
- spot Sp1 76% 43%
- spot Sp2 30% 52%
+ above 60 °C
```

# Temperature data

```@thermal
data: ../images/thermal/cabinet-after.thermal.png
- spot Sp1 76% 43%
- spot Sp2 30% 52%
+ above 35 °C
```

# A known scalar ramp

```@thermal
data: ../images/thermal/ramp.thermal.png
palette: white-hot
- spot 25 25% 50%
- spot 75 75% 50%
```

# Before and after, on one scale
@layout: two-column
@thermal-window: 25..90 °C

```@thermal
image: ../images/thermal/cabinet-before.png
mapping: linear 18..92 °C
label: Before
- spot B3 76% 43%
```

+++

```@thermal
data: ../images/thermal/cabinet-after.thermal.png
label: After
- spot B3 76% 43%
```
