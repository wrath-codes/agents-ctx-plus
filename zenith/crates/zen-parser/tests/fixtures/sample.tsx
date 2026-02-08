interface ButtonProps {
    label: string;
    onClick: () => void;
    disabled?: boolean;
}

export function Button({ label, onClick, disabled = false }: ButtonProps): JSX.Element {
    return <button onClick={onClick} disabled={disabled}>{label}</button>;
}

export default function App(): JSX.Element {
    return <Button label="Click me" onClick={() => {}} />;
}
