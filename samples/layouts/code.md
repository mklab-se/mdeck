---
title: "Layout Test: Code Slides"
@theme: dark
@transition: fade
---

# Layout Test: Code Slides
Focused tests for the code layout


# Hello World in Rust

```rust
fn main() {
    println!("Hello, world!");
}
```


# Python with Line Highlights

```python {2,4-5}
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)
# This line is also highlighted

print(fibonacci(10))
```


# Longer Code Block

```javascript
class PresentationEngine {
    constructor(slides) {
        this.slides = slides;
        this.currentIndex = 0;
        this.transition = 'fade';
    }

    next() {
        if (this.currentIndex < this.slides.length - 1) {
            this.currentIndex++;
            this.render();
        }
    }

    previous() {
        if (this.currentIndex > 0) {
            this.currentIndex--;
            this.render();
        }
    }

    render() {
        const slide = this.slides[this.currentIndex];
        console.log(`Rendering slide: ${slide.title}`);
    }
}
```

---

# Long code shrinks to fit

```rust
fn main() {
    let value_1 = compute_something(1); // line 1
    let value_2 = compute_something(2); // line 2
    let value_3 = compute_something(3); // line 3
    let value_4 = compute_something(4); // line 4
    let value_5 = compute_something(5); // line 5
    let value_6 = compute_something(6); // line 6
    let value_7 = compute_something(7); // line 7
    let value_8 = compute_something(8); // line 8
    let value_9 = compute_something(9); // line 9
    let value_10 = compute_something(10); // line 10
    let value_11 = compute_something(11); // line 11
    let value_12 = compute_something(12); // line 12
    let value_13 = compute_something(13); // line 13
    let value_14 = compute_something(14); // line 14
    let value_15 = compute_something(15); // line 15
    let value_16 = compute_something(16); // line 16
    let value_17 = compute_something(17); // line 17
    let value_18 = compute_something(18); // line 18
    let value_19 = compute_something(19); // line 19
    let value_20 = compute_something(20); // line 20
    let value_21 = compute_something(21); // line 21
    let value_22 = compute_something(22); // line 22
    let value_23 = compute_something(23); // line 23
    let value_24 = compute_something(24); // line 24
    let value_25 = compute_something(25); // line 25
    let value_26 = compute_something(26); // line 26
    let value_27 = compute_something(27); // line 27
    let value_28 = compute_something(28); // line 28
    let value_29 = compute_something(29); // line 29
    let value_30 = compute_something(30); // line 30
    let value_31 = compute_something(31); // line 31
    let value_32 = compute_something(32); // line 32
    let value_33 = compute_something(33); // line 33
    let value_34 = compute_something(34); // line 34
    let value_35 = compute_something(35); // line 35
    let value_36 = compute_something(36); // line 36
    let value_37 = compute_something(37); // line 37
    let value_38 = compute_something(38); // line 38
    let value_39 = compute_something(39); // line 39
    let value_40 = compute_something(40); // line 40
}
```

---

# Long lines shrink instead of wrapping

```python
def a_function_with_a_rather_long_signature(first_argument, second_argument, third_argument, fourth):
    return first_argument + second_argument + third_argument + fourth  # and a trailing comment
```
