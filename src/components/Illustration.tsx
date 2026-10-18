const assets = import.meta.glob('../assets/illustrations/*.png', { eager: true, query: '?url', import: 'default' }) as Record<string, string>;
export default function Illustration({ name, className = '' }: { name: 'storage' | 'migration' | 'cleanup' | 'tools' | 'scan' | 'startup'; className?: string }) {
  return <img className={`illustration ${className}`} src={assets[`../assets/illustrations/${name}.png`]} alt="" aria-hidden="true" draggable={false} />;
}
