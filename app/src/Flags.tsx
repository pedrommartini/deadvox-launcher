export function Flag({ country, small = false }: { country: 'us' | 'br'; small?: boolean }) {
  return (
    <span className={`flag ${small ? 'flag-small' : ''}`}>
      {country === 'us' ? <svg viewBox="0 0 190 100" role="img" aria-label="Bandeira dos Estados Unidos">
        <defs><path id="star" d="M0,-3 0.7,-0.9 2.85,-0.9 1.1,0.35 1.75,2.4 0,1.15 -1.75,2.4 -1.1,0.35 -2.85,-0.9 -0.7,-0.9Z" /></defs>
        <rect width="190" height="100" fill="#f4f1eb" />
        {Array.from({ length: 7 }, (_, i) => <rect key={i} y={i * 100 / 6.5} width="190" height={100 / 13} fill="#ab3540" />)}
        <rect width="76" height={700 / 13} fill="#273955" />
        {Array.from({ length: 9 }, (_, row) => Array.from({ length: row % 2 ? 5 : 6 }, (_, col) =>
          <use key={`${row}-${col}`} href="#star" transform={`translate(${(row % 2 ? 12.65 : 6.35) + col * 12.65},${5.3 + row * 5.38})`} fill="#f4f1eb" />
        ))}
      </svg> : <svg viewBox="0 0 190 100" role="img" aria-label="Bandeira do Brasil">
        <rect width="190" height="100" fill="#16813a" />
        <path d="M95 10 174 50 95 90 16 50Z" fill="#f2c93e" />
        <circle cx="95" cy="50" r="25" fill="#284777" />
        <path d="M71.3 43.7Q97 41 119.2 55" fill="none" stroke="#f6f2e5" strokeWidth="5.1" />
        {[[87,54],[96,67],[107,57],[80,46],[94,61],[100,49],[110,66],[89,70],[104,72]].map(([x,y]) =>
          <circle key={`${x}-${y}`} cx={x} cy={y} r="0.8" fill="#f7f4e9" />
        )}
      </svg>}
    </span>
  );
}
