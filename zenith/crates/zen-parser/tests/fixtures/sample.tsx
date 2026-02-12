"use client";

import React, { useState, useEffect, useRef, useMemo, useCallback, useReducer, createContext, useContext, Suspense } from 'react';

// ── Context ────────────────────────────────────────────────────────
interface ThemeContextValue {
    theme: 'light' | 'dark';
    toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextValue | undefined>(undefined);

// ── Custom hooks ───────────────────────────────────────────────────

/** Custom hook for theme access */
export function useTheme(): ThemeContextValue {
    const ctx = useContext(ThemeContext);
    if (!ctx) throw new Error('useTheme must be used within ThemeProvider');
    return ctx;
}

/** Fetch data from an API endpoint */
export function useFetch<T>(url: string): { data: T | null; loading: boolean; error: Error | null } {
    const [data, setData] = useState<T | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<Error | null>(null);

    useEffect(() => {
        fetch(url)
            .then(res => res.json())
            .then(setData)
            .catch(setError)
            .finally(() => setLoading(false));
    }, [url]);

    return { data, loading, error };
}

// ── Props types ────────────────────────────────────────────────────

interface ButtonProps {
    label: string;
    onClick: () => void;
    disabled?: boolean;
    variant?: 'primary' | 'secondary';
    children?: React.ReactNode;
}

interface UserCardProps {
    user: { id: number; name: string; email: string };
    onSelect?: (id: number) => void;
}

interface ListProps<T> {
    items: T[];
    renderItem: (item: T, index: number) => React.ReactNode;
    keyExtractor: (item: T) => string;
}

type CardProps = {
    title: string;
    subtitle?: string;
};

interface AvatarProps {
    src: string;
    alt?: string;
}

// ── Function components ────────────────────────────────────────────

/**
 * Primary button component.
 * @param props - Button properties
 */
export function Button({ label, onClick, disabled = false, variant = 'primary', children }: ButtonProps): JSX.Element {
    return (
        <button
            className={`btn btn-${variant}`}
            onClick={onClick}
            disabled={disabled}
            data-testid="button"
        >
            {children ?? label}
        </button>
    );
}

/** User profile card */
export const UserCard: React.FC<UserCardProps> = ({ user, onSelect }) => {
    const handleClick = useCallback(() => {
        onSelect?.(user.id);
    }, [onSelect, user.id]);

    return (
        <div className="user-card" onClick={handleClick}>
            <h3>{user.name}</h3>
            <p>{user.email}</p>
        </div>
    );
};

/** Generic list component */
export function List<T>({ items, renderItem, keyExtractor }: ListProps<T>): JSX.Element {
    return (
        <ul>
            {items.map((item, i) => (
                <li key={keyExtractor(item)}>{renderItem(item, i)}</li>
            ))}
        </ul>
    );
}

/** Counter with multiple hooks */
export function Counter(): JSX.Element {
    const [count, setCount] = useState(0);
    const prevCount = useRef(count);
    const doubled = useMemo(() => count * 2, [count]);

    useEffect(() => {
        prevCount.current = count;
    }, [count]);

    return (
        <div>
            <p>Count: {count}</p>
            <p>Previous: {prevCount.current}</p>
            <p>Doubled: {doubled}</p>
            <Button label="Increment" onClick={() => setCount(c => c + 1)} />
            <Button label="Decrement" onClick={() => setCount(c => c - 1)} />
        </div>
    );
}

// ── Private (non-exported) component ───────────────────────────────

function Sidebar(): JSX.Element {
    return <nav className="sidebar">sidebar</nav>;
}

// ── Reducer component ──────────────────────────────────────────────

interface TodoState {
    todos: string[];
    count: number;
}

type TodoAction = { type: 'add'; text: string } | { type: 'clear' };

/** Todo list with useReducer */
export function TodoApp(): JSX.Element {
    const [state, dispatch] = useReducer(
        (s: TodoState, a: TodoAction) => s,
        { todos: [], count: 0 }
    );

    return (
        <div>
            <p>{state.count} todos</p>
            <button onClick={() => dispatch({ type: 'clear' })}>Clear</button>
        </div>
    );
}

// ── Default export ─────────────────────────────────────────────────

/** Main application shell */
export default function App(): JSX.Element {
    const [theme, setTheme] = useState<'light' | 'dark'>('light');
    const toggleTheme = useCallback(() => {
        setTheme(t => t === 'light' ? 'dark' : 'light');
    }, []);

    return (
        <ThemeContext.Provider value={{ theme, toggleTheme }}>
            <div className={`app ${theme}`}>
                <h1>My App</h1>
                <Counter />
                <Sidebar />
                <UserCard user={{ id: 1, name: 'Alice', email: 'a@b.com' }} />
            </div>
        </ThemeContext.Provider>
    );
}

// ── forwardRef ─────────────────────────────────────────────────────

interface InputProps {
    value: string;
    onChange: (val: string) => void;
    placeholder?: string;
}

export const FancyInput = React.forwardRef<HTMLInputElement, InputProps>(
    ({ value, onChange, placeholder }, ref) => (
        <input
            ref={ref}
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder={placeholder}
            className="fancy-input"
        />
    )
);

// ── React.memo ─────────────────────────────────────────────────────

export const MemoCard = React.memo(function MemoCardInner({ title, subtitle }: CardProps) {
    return (
        <div className="card">
            <h2>{title}</h2>
            {subtitle && <p>{subtitle}</p>}
        </div>
    );
});

export const MemoAvatar = React.memo(({ src, alt }: AvatarProps) => (
    <img src={src} alt={alt} className="avatar" />
));

// ── React.lazy + Suspense ──────────────────────────────────────────

const LazySettings = React.lazy(() => import('./Settings'));

/** Page with suspense boundary */
export function PageWithSuspense(): JSX.Element {
    return (
        <div>
            <h1>Settings</h1>
            <Suspense fallback={<div>Loading...</div>}>
                <LazySettings />
            </Suspense>
        </div>
    );
}

// ── Higher-order component ─────────────────────────────────────────

export function withLoading<P extends object>(
    Component: React.ComponentType<P>
): React.FC<P & { loading: boolean }> {
    return function WithLoadingWrapper({ loading, ...rest }: P & { loading: boolean }) {
        if (loading) return <div className="spinner">Loading...</div>;
        return <Component {...(rest as P)} />;
    };
}

// ── Class components ───────────────────────────────────────────────

interface EBProps {
    children: React.ReactNode;
    fallback?: React.ReactNode;
}

interface EBState {
    hasError: boolean;
}

/** Error boundary class component */
export class ErrorBoundary extends React.Component<EBProps, EBState> {
    constructor(props: EBProps) {
        super(props);
        this.state = { hasError: false };
    }

    static getDerivedStateFromError(_error: Error): EBState {
        return { hasError: true };
    }

    componentDidCatch(error: Error, info: React.ErrorInfo): void {
        console.error(error, info);
    }

    render(): React.ReactNode {
        if (this.state.hasError) {
            return this.props.fallback ?? <div>Something went wrong</div>;
        }
        return this.props.children;
    }
}

interface CounterClassProps {
    initial: number;
}

class PureCounter extends React.PureComponent<CounterClassProps> {
    render(): React.ReactNode {
        return <div>Count: {this.props.initial}</div>;
    }
}

// ── Non-component exports ──────────────────────────────────────────

export const API_URL = 'https://api.example.com';

export function formatDate(date: Date): string {
    return date.toISOString().split('T')[0];
}

export type Theme = 'light' | 'dark';

export enum Status {
    Active = 'active',
    Inactive = 'inactive',
}
