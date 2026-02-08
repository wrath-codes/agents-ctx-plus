defmodule Sample.Processor do
  @moduledoc "A sample processor module."

  @doc "Process a list of items."
  def process(items) when is_list(items) do
    Enum.map(items, &transform/1)
  end

  defp transform(item) do
    String.upcase(item)
  end

  defmacro define_handler(name) do
    quote do
      def unquote(name)(), do: :ok
    end
  end
end
