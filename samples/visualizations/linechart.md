---
title: "Line Chart Tests"
@theme: dark
---

# Line Chart — Single Series

```@linechart
# x-labels: Q1, Q2, Q3, Q4
- Revenue: 100, 150, 200, 280
```


# Line Chart — With Axis Labels

```@linechart
# x-labels: Jan, Feb, Mar, Apr, May, Jun
# x-label: Month
# y-label: Temperature (°C)
- London: 5, 6, 10, 14, 17, 20
- Madrid: 10, 12, 16, 19, 23, 28
```


# Line Chart — Multiple Series

```@linechart
# x-labels: 2020, 2021, 2022, 2023, 2024
# x-label: Year
# y-label: Users (millions)
- Product A: 10, 25, 45, 80, 120
+ Product B: 5, 15, 30, 55, 90
+ Product C: 2, 8, 20, 40, 70
```


# Line Chart — Progressive Reveal

```@linechart
# x-labels: Mon, Tue, Wed, Thu, Fri
# y-label: Requests (k)
- API v1: 120, 115, 130, 125, 140
+ API v2: 80, 95, 110, 130, 160
* Legacy: 40, 35, 30, 25, 20
```


---

# Line Chart — Many Points (Label Thinning) and Thousands

```@linechart
# x-labels: Jan 2023, Feb 2023, Mar 2023, Apr 2023, May 2023, Jun 2023, Jul 2023, Aug 2023, Sep 2023, Oct 2023, Nov 2023, Dec 2023, Jan 2024, Feb 2024, Mar 2024, Apr 2024, May 2024, Jun 2024, Jul 2024, Aug 2024, Sep 2024, Oct 2024, Nov 2024, Dec 2024
# x-label: Month
# y-label: Revenue ($)
- Revenue: 12,000, 13,500, 15,200, 14,800, 16,900, 18,400, 19,100, 21,000, 22,500, 24,300, 26,800, 29,000, 27,500, 28,900, 31,200, 33,400, 35,100, 36,800, 38,200, 40,500, 42,900, 45,600, 47,200, 50,100
- Costs: 9,000, 9,400, 9,900, 10,200, 10,800, 11,300, 11,900, 12,400, 12,800, 13,500, 14,100, 14,900, 15,200, 15,800, 16,300, 16,900, 17,400, 18,000, 18,500, 19,100, 19,800, 20,400, 21,000, 21,700
```
