type LevelProps = {
  depth: number;
  name: string;
};

export function Level({ depth, name }: LevelProps) {
  return (
    <li data-depth={depth}>
      <strong>{name}</strong>
    </li>
  );
}
