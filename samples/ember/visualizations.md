---
title: "A year in numbers"
author: "MKLab"
@theme: ember
@transition: fade
---

# A year in numbers

What the platform team shipped, measured, and learned in twelve months.

---

## Where the time went

```@barchart
# y-label: Weeks
- Platform: 18
- Features: 14
- Incidents: 6
- Hiring: 5
- Training: 9
```

---

## Deploys per week

```@linechart
# x-label: Quarter
- Deploys: 12, 19, 27, 41
- Rollbacks: 3, 2, 2, 1
```

---

## How the system fits together

```@architecture
- Clients      (icon: browser,   pos: 1,1)
- Edge         (icon: cloud,     pos: 2,1)
- API          (icon: function,  pos: 3,1)
- Queue        (icon: queue,     pos: 3,2)
- Workers      (icon: container, pos: 4,2)
- Store        (icon: database,  pos: 4,1)

- Clients -> Edge: requests
- Edge -> API: routes
- API -> Store: reads and writes
- API -> Queue: events
- Queue -> Workers: jobs
- Workers -> Store: results
```

---

## Who owns what

```@piechart
- Platform: 40
- Product A: 30
- Product B: 20
- Shared: 10
```

---

## Milestones

```@timeline
- 2026-01: Platform team formed
- 2026-03: First automated deploy
- 2026-06: Zero-downtime migrations
- 2026-09: Self-service environments
- 2026-12: On-call rotation halved
```

---

## Delivery plan

```@gantt
- Discovery: 2026-10-01, 2 weeks
- Design: 2026-10-15, 3 weeks, after Discovery
- Build: 2026-11-05, 6 weeks, after Design
- Pilot: 2026-12-17, 3 weeks, after Build
- Rollout: 2027-01-07, 4 weeks, after Pilot
```

---

## Health

```@kpi
- Uptime: 99.97% (trend: +0.12%)
- Lead time: 2.1 days (trend: -1.4 days)
- Change failure: 4% (trend: -3%)
- MTTR: 38 min (trend: -22 min)
```

---

# Thank you

The numbers are the easy part. The habits behind them are the talk.
