# Sample Elixir file exercising all constructs for extractor tests.

defmodule Sample.Processor do
  @moduledoc """
  A sample processor module.

  Handles item processing with configurable strategies.
  """

  @doc """
  Process a list of items.

  ## Examples

      iex> Sample.Processor.process(["a", "b"])
      ["A", "B"]
  """
  @spec process(list(String.t())) :: list(String.t())
  def process(items) when is_list(items) do
    Enum.map(items, &transform/1)
  end

  @doc "Process a single item."
  def process_one(item), do: transform(item)

  @doc false
  def internal_helper(x), do: x

  defp transform(item) do
    String.upcase(item)
  end

  defp validate(item, opts \\ []) do
    case Keyword.get(opts, :strict, false) do
      true -> is_binary(item)
      false -> true
    end
  end

  @doc "Define a handler at compile time."
  defmacro define_handler(name) do
    quote do
      def unquote(name)(), do: :ok
    end
  end

  defmacrop internal_macro(expr) do
    quote do
      IO.inspect(unquote(expr))
    end
  end
end

# ── Struct module ──────────────────────────────────────────────────

defmodule Sample.Config do
  @moduledoc "Configuration struct."

  @enforce_keys [:name]
  defstruct name: nil, retries: 3, timeout: 5_000

  @type t :: %__MODULE__{
          name: String.t(),
          retries: non_neg_integer(),
          timeout: pos_integer()
        }

  @doc "Create a new Config with defaults."
  def new(name, opts \\ []) do
    %__MODULE__{
      name: name,
      retries: Keyword.get(opts, :retries, 3),
      timeout: Keyword.get(opts, :timeout, 5_000)
    }
  end

  @doc "Run with this config."
  def run(%__MODULE__{} = config) do
    {:ok, config.name}
  end
end

# ── GenServer module ──────────────────────────────────────────────

defmodule Sample.Worker do
  @moduledoc "A GenServer worker."

  use GenServer

  @doc "Start the worker."
  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc "Get the current state."
  def get_state do
    GenServer.call(__MODULE__, :get_state)
  end

  @impl true
  def init(opts) do
    {:ok, %{count: 0, opts: opts}}
  end

  @impl true
  def handle_call(:get_state, _from, state) do
    {:reply, state, state}
  end

  @impl true
  def handle_cast({:increment, n}, state) do
    {:noreply, %{state | count: state.count + n}}
  end

  @impl true
  def handle_info(:tick, state) do
    {:noreply, %{state | count: state.count + 1}}
  end
end

# ── Protocol ──────────────────────────────────────────────────────

defprotocol Sample.Renderable do
  @moduledoc "A protocol for rendering items."

  @doc "Render the item to a string."
  def render(item)
end

defimpl Sample.Renderable, for: BitString do
  def render(item), do: item
end

# ── Module with callbacks ─────────────────────────────────────────

defmodule Sample.Behaviour do
  @moduledoc "A behaviour module."

  @callback handle_event(event :: term()) :: :ok | {:error, term()}
  @callback format_output(data :: map()) :: String.t()

  @optional_callbacks [format_output: 1]
end

# ── Module with guards and multi-clause ───────────────────────────

defmodule Sample.Guards do
  @moduledoc "Module exercising guards and multi-clause functions."

  @doc "Classify a value by type."
  def classify(x) when is_integer(x), do: :integer
  def classify(x) when is_float(x), do: :float
  def classify(x) when is_binary(x), do: :string
  def classify(_), do: :unknown

  @doc "Add two numbers."
  def add(a, b) when is_number(a) and is_number(b), do: a + b
end

# ── Module with types and specs ───────────────────────────────────

defmodule Sample.Types do
  @moduledoc "Module with type definitions."

  @type direction :: :north | :south | :east | :west
  @type result(ok, err) :: {:ok, ok} | {:error, err}
  @typep internal_state :: %{count: integer(), name: String.t()}
  @opaque wrapped :: {atom(), term()}

  @spec transform(direction()) :: String.t()
  def transform(:north), do: "N"
  def transform(:south), do: "S"
  def transform(:east), do: "E"
  def transform(:west), do: "W"
end

# ── Module with constants ─────────────────────────────────────────

defmodule Sample.Constants do
  @moduledoc "Module with module attributes as constants."

  @max_retries 3
  @default_timeout 5_000
  @version "1.0.0"

  def max_retries, do: @max_retries
  def default_timeout, do: @default_timeout
end

# ── Exception module ──────────────────────────────────────────────

defmodule Sample.AppError do
  @moduledoc "Application error."

  defexception [:message, :code]

  @doc "Create from a status code."
  def from_code(code) do
    %__MODULE__{message: "Error #{code}", code: code}
  end
end

# ── Guards module ─────────────────────────────────────────────────

defmodule Sample.CustomGuards do
  @moduledoc "Custom guard definitions."

  @doc "Check if value is a positive integer."
  defguard is_pos_integer(value) when is_integer(value) and value > 0

  defguardp is_internal(value) when is_atom(value) and value != nil
end

# ── Delegate module ───────────────────────────────────────────────

defmodule Sample.Delegator do
  @moduledoc "Module with delegated functions."

  defdelegate process(items), to: Sample.Processor
  defdelegate new(name, opts \\ []), to: Sample.Config
end

# ── Unexported / private module ───────────────────────────────────

defmodule Sample.Internal do
  @moduledoc false

  def helper(x), do: x * 2
end
