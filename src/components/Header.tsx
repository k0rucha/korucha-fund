import Link from "next/link";
import ThemeToggle from "@/components/ThemeToggle";
import RefreshButton from "@/components/RefreshButton";

export default function Header() {
  return (
    <header className="mb-6 flex items-center justify-between border-b-4 border-da-orange-600 pb-3">
      <Link href="/" className="text-2xl font-black text-da-blue-1200">
        こるちゃファンド
      </Link>
      <div className="flex items-center gap-3">
        <Link href="/status" className="text-sm text-da-blue-900 hover:underline">
          ステータス
        </Link>
        <Link href="/admin" className="text-sm text-da-blue-900 hover:underline">
          管理
        </Link>
        <RefreshButton />
        <ThemeToggle />
      </div>
    </header>
  );
}
