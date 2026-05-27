import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent, waitFor, act } from '@testing-library/react';
import type { DirectoryUser } from '../app/lib/directory';

vi.mock('../app/lib/actions', () => ({
  searchDirectoryAction: vi.fn(),
}));

vi.mock('../app/components/spill-ui', () => ({
  Tile: ({ children, className }: { children: React.ReactNode; className?: string }) => (
    <div className={className}>{children}</div>
  ),
  fieldControlClass: 'input',
  Avatar: () => null,
  avatarInitials: (name: string) => name.slice(0, 2),
  avatarColorForSeed: () => '#aaa',
}));

import { searchDirectoryAction } from '../app/lib/actions';
import { UserAutocomplete } from '../app/components/user-autocomplete';

const mockedSearch = vi.mocked(searchDirectoryAction);

describe('UserAutocomplete', () => {
  const onPick = vi.fn();

  beforeEach(() => {
    onPick.mockReset();
    mockedSearch.mockReset();
  });

  it('renders an input with a placeholder', () => {
    render(<UserAutocomplete onPick={onPick} />);
    expect(screen.getByPlaceholderText('Search by name or email...')).toBeInTheDocument();
  });

  it('does not call searchDirectoryAction when query length < 2', async () => {
    render(<UserAutocomplete onPick={onPick} />);
    const input = screen.getByPlaceholderText('Search by name or email...');

    fireEvent.change(input, { target: { value: 'a' } });

    // Give any pending async work a tick to surface
    await act(async () => {});
    expect(mockedSearch).not.toHaveBeenCalled();
  });

  it('calls searchDirectoryAction with the typed query when length >= 2', async () => {
    mockedSearch.mockResolvedValue([]);
    render(<UserAutocomplete onPick={onPick} />);
    const input = screen.getByPlaceholderText('Search by name or email...');

    fireEvent.change(input, { target: { value: 'al' } });

    await waitFor(() => expect(mockedSearch).toHaveBeenCalledWith('al'));
  });

  it('shows results returned by searchDirectoryAction', async () => {
    const users: DirectoryUser[] = [
      { email: 'alice@example.com', name: 'Alice' },
      { email: 'alicia@example.com', name: 'Alicia' },
    ];
    mockedSearch.mockResolvedValue(users);
    render(<UserAutocomplete onPick={onPick} />);
    const input = screen.getByPlaceholderText('Search by name or email...');

    fireEvent.change(input, { target: { value: 'ali' } });

    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
      expect(screen.getByText('Alicia')).toBeInTheDocument();
    });
  });

  it('calls onPick with the selected user and clears the input', async () => {
    const users: DirectoryUser[] = [{ email: 'alice@example.com', name: 'Alice' }];
    mockedSearch.mockResolvedValue(users);
    render(<UserAutocomplete onPick={onPick} />);
    const input = screen.getByPlaceholderText('Search by name or email...');

    fireEvent.change(input, { target: { value: 'ali' } });

    await waitFor(() => screen.getByText('Alice'));
    fireEvent.click(screen.getByText('Alice'));

    expect(onPick).toHaveBeenCalledWith([{ email: 'alice@example.com', name: 'Alice' }]);
    expect(input).toHaveValue('');
  });

  it('clears results when query drops below 2 characters', async () => {
    const users: DirectoryUser[] = [{ email: 'alice@example.com', name: 'Alice' }];
    mockedSearch.mockResolvedValue(users);
    render(<UserAutocomplete onPick={onPick} />);
    const input = screen.getByPlaceholderText('Search by name or email...');

    fireEvent.change(input, { target: { value: 'ali' } });
    await waitFor(() => screen.getByText('Alice'));

    fireEvent.change(input, { target: { value: 'a' } });
    await waitFor(() => expect(screen.queryByText('Alice')).not.toBeInTheDocument());
  });
});
