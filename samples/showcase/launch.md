---
title: "Northwind"
author: "MDeck"
theme: ember
---

# Ship It
<!-- picture: rocket -->

The Northwind launch, written in plain markdown

# Why we are building it
<!-- picture: lightbulb -->

- Teams lose a day a week to status meetings
- Every tool shows a different truth
- We make the plan the single source

# Launch week

```@kpi
- Sign-ups: 12.4K (trend: +38%)
- Activation: 64% (trend: +9%)
- Churn: 1.8% (trend: -0.6%)
- NPS: 71 (trend: +12)
```

# How it fits together

```@architecture
- Browser   (icon: browser,  pos: 1,2)
- Gateway   (icon: api,      pos: 2,2)
- Auth      (icon: lock,     pos: 3,1)
- Planner   (icon: function, pos: 3,2)
- Events    (icon: queue,    pos: 3,3)
- Postgres  (icon: database, pos: 4,2)

- Browser -> Gateway: HTTPS
- Gateway -> Auth: tokens
- Gateway -> Planner: requests
- Planner -> Postgres: queries
- Planner -> Events: publishes
```

# Growth by quarter

```@stackedbar
# categories: Q1, Q2, Q3, Q4
- Teams: 12, 28, 46, 71
- Seats: 30, 55, 88, 120
- Integrations: 8, 19, 31, 52
```

# Launch schedule

| Time  | Milestone       | Team    |
|-------|-----------------|---------|
| 09:00 | Doors open      | Ops     |
| 10:30 | Keynote         | All     |
| 13:00 | Public beta     | Product |
| 15:45 | Press briefing  | Comms   |

# Platform 4

![The evening train](station.jpg)

- Night train to Paris
- Departs 18:42
- Boarding now

# Built to last
<!-- picture: gear -->

- Every part replaceable
- Every change measured
- Nothing left to chance
