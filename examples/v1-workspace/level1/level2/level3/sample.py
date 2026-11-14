from dataclasses import dataclass


@dataclass(frozen=True)
class Level:
    depth: int
    name: str

    def label(self) -> str:
        return f"{self.depth}: {self.name}"


if __name__ == "__main__":
    print(Level(3, "level3").label())
