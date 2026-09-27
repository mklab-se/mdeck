Modal overlay for focused decisions. Blurs the room behind it. Controlled via `open`/`onClose`; Esc closes.

```jsx
<Dialog open={open} onClose={close} title="Request access"
  description="Tell us who you are. We reply within two business days."
  footer={<><Button variant="ghost" onClick={close}>Cancel</Button><Button>Send</Button></>}>
  <Input label="Work email" />
</Dialog>
```
