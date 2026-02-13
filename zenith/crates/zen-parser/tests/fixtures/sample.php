<?php

namespace App\Core;

use Psr\Log\LoggerInterface;
use function strlen as str_len;
use const PHP_VERSION_ID;
use Vendor\Tools\{Formatter as Fmt, Runner};

const TOP_LIMIT = 10;

/**
 * @template T
 * @param T $value
 * @return T
 */
function identity($value) {
    return $value;
}

$arrow = fn (int $x): int => $x + 1;
$closure = static function (string $name) use ($arrow): string {
    return $name . $arrow(1);
};

function globalState(): void {
    global $globalCounter;
    static $memo = [];
}

interface Renderable {
    public function render(): string;
}

trait UsesHelpers {
    public function helper(): string {
        return 'ok';
    }
}

enum Status: string {
    case Ready = 'ready';
    case Done = 'done';
}

class Service implements Renderable {
    use UsesHelpers;

    public const VERSION = '1.0';

    protected string $name;

    public string $slug {
        get => $this->name;
        set(string $value) {
            $this->name = $value;
        }
    }

    public function __construct(private int $id, string $name) {
        $this->name = $name;
        $tmp = new class($name) {
            public function __construct(private string $name) {}
            public function value(): string {
                return $this->name;
            }
        };
    }

    /**
     * Render service state.
     * @param string $suffix
     * @return string
     */
    public function render(string|int $suffix = ''): string|int {
        return $this->name . $suffix;
    }
}
