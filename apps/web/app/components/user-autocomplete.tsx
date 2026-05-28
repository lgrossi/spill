'use client';

import { useTransition, useState, useRef, useEffect } from 'react';
import { searchDirectoryAction } from '@/lib/actions';
import type { DirectoryEntry } from '@/lib/directory';
import { Tile, fieldControlClass, Avatar, avatarInitials, avatarColorForSeed } from './spill-ui';

export type Picked = { email: string; name: string; role?: "host" | "member" };

export function UserAutocomplete({ onPick }: { onPick: (users: Picked[]) => void }) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<DirectoryEntry[]>([]);
  const [isPending, startTransition] = useTransition();
  const latestQuery = useRef('');
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (query.length < 2) {
      setResults([]);
      return;
    }
    if (debounceTimer.current) clearTimeout(debounceTimer.current);
    debounceTimer.current = setTimeout(() => {
      const snapshot = query;
      startTransition(async () => {
        const users = await searchDirectoryAction(snapshot);
        if (latestQuery.current === snapshot) setResults(users);
      });
    }, 300);
    return () => { if (debounceTimer.current) clearTimeout(debounceTimer.current); };
  }, [query]);

  function handleChange(e: React.ChangeEvent<HTMLInputElement>) {
    const q = e.target.value;
    latestQuery.current = q;
    setQuery(q);
  }

  function pick(entry: DirectoryEntry) {
    if (entry.members && entry.members.length > 0) {
      onPick(entry.members.map((email) => ({ email, name: email })));
    } else {
      onPick([{ email: entry.email, name: entry.name }]);
    }
    setQuery('');
    setResults([]);
  }

  return (
    <div className="relative">
      <input
        className={`${fieldControlClass} w-full`}
        onChange={handleChange}
        placeholder="Search by name or email..."
        type="text"
        value={query}
      />
      {results.length > 0 && (
        <Tile className="absolute left-0 right-0 top-full z-10 mt-1 flex flex-col gap-0.5 p-1">
          {results.map((user) => (
            <button
              className="flex w-full items-center gap-2.5 rounded-[7px] px-2 py-1.5 text-left hover:bg-[var(--panel-hi)] focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
              key={user.email}
              onClick={() => pick(user)}
              type="button"
            >
              <Avatar
                color={avatarColorForSeed(user.email)}
                k={avatarInitials(user.name || user.email)}
                size={24}
              />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[12.5px] font-semibold text-spill-fg">{user.name}</div>
                <div className="truncate text-[11px] text-spill-muted">
                  {user.members ? `Group · ${user.members.length} member${user.members.length === 1 ? '' : 's'}` : user.email}
                </div>
              </div>
            </button>
          ))}
        </Tile>
      )}
      {isPending && query.length >= 2 && results.length === 0 && (
        <div className="mt-1 text-[11px] text-spill-muted">Searching...</div>
      )}
    </div>
  );
}
