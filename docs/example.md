Example
=======

```text
(interface fruit
  (fn seasonal () bool)
  (fn set_expires (int) void)
  (fn sweet () bool))

(class shovel
  (ctor (int int string))
  (fn dig () void)
  (prop materials (vector string)))

(class spade shovel
  (ctor (int int string))
  (ctor (int int string string)))

(class apple fruit
  (prop color string)
  (private
    (prop has_worms bool))
  (ctor ((color string)))
  (fn rot () void))
```

produces the UML diagram:

![](example.png)
