# Level 3 reference

Folder three of five. Python source sits beside this file and folder four.

```python
from dataclasses import dataclass


@dataclass(frozen=True)
class Level:
    depth: int
    name: str

    def label(self) -> str:
        return f"{self.depth}: {self.name}"


print(Level(3, "level3").label())
```

| Feature | Status |
|---------|:------:|
| Tables | ready |
| LaTeX | ready |
| Images | ready |
| Mermaid | ready |
