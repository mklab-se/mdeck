---
title: "Bar Chart Tests"
@theme: dark
---

# Bar Chart — Vertical

```@barchart
- JavaScript: 65
- Python: 48
- TypeScript: 38
- Rust: 22
- Go: 28
```


# Bar Chart — Vertical with Axis Labels

```@barchart
# x-label: Programming Language
# y-label: Popularity Index
- JavaScript: 65
- Python: 48
- TypeScript: 38
- Rust: 22
- Go: 28
- Java: 42
```


# Bar Chart — Horizontal

```@barchart
# orientation: horizontal
- Revenue: 420
- Expenses: 310
- Profit: 110
- Investments: 85
```


# Bar Chart — Horizontal with Axis Labels

```@barchart
# orientation: horizontal
# x-label: Amount ($M)
# y-label: Category
- Revenue: 420
- Expenses: 310
- Profit: 110
- Investments: 85
- R&D: 65
```


# Bar Chart — Progressive Reveal

```@barchart
# y-label: Performance Score
- Rust: 95
+ C++: 90
+ Go: 72
* Java: 60
+ Python: 35
```


# Bar Chart — Many Items

```@barchart
# x-label: Country
# y-label: GDP ($T)
- USA: 25.5
- China: 18.3
- Japan: 4.2
- Germany: 4.1
- UK: 3.1
- India: 3.0
- France: 2.8
- Canada: 2.1
- Italy: 2.0
- Brazil: 1.9
```


# Bar Chart — Long Labels and Decorated Values

```@barchart
# y-label: Revenue
- Enterprise Software Licensing: $1,250,000
- Professional Services and Consulting: $840,500
- Cloud Infrastructure Subscriptions: $2,100,000
- Training and Certification Programs: $310,000
- Hardware Resale: $95,000
- Support Contracts: $640,000
- Marketplace Fees: $120,000
- Other Income: $45,000
```


# Bar Chart — Horizontal with Long Labels

```@barchart
# orientation: horizontal
- Customer Acquisition Cost per Enterprise Segment: 48 units
- Net Revenue Retention: 112%
- Gross Margin: 71%
- Average Contract Value Growth Year over Year Compared to Plan: 24%
- Churn: 3.5%
```
