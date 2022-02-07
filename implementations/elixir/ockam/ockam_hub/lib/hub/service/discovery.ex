
defmodule Ockam.Hub.Service.Discovery.ServiceInfo do
  @moduledoc """
  Service info structure for discovery service.
  """
  defstruct [:id, :route, metadata: %{}]

  @type t() :: %__MODULE__{
          id: binary(),
          route: [Ockam.Address.t()],
          metadata: %{binary() => binary()}
        }
end

defmodule Ockam.Hub.Service.Discovery do
  @moduledoc """
  Discovery service storing information about other services

  Options:
  storage: storage module to use, default is `Ockam.Hub.Service.Discovery.Storage.Memory`
  """

  use Ockam.Worker

  alias Ockam.Bare.Extended, as: BareExtended
  alias Ockam.Hub.Service.Discovery.ServiceInfo
  alias Ockam.Message
  alias Ockam.Router

  require Logger

  def register_service(registry_route, id, route, metadata \\ %{}) do
    Logger.info("Registering #{inspect(id)}, #{inspect(route)} #{inspect(metadata)}")
    payload = encode_register_request(id, metadata)
    Router.route(payload, registry_route, route)
  end

  @impl true
  def setup(options, state) do
    storage = Keyword.get(options, :storage, Ockam.Hub.Service.Discovery.Storage.Memory)

    {:ok, Map.put(state, :storage, {storage, storage.init()})}
  end

  @impl true
  def handle_message(message, state) do
    result =
      case parse_request(message) do
        :list ->
          list(state)

        {:get, id} ->
          get(id, state)

        {:register, id, route, metadata} ->
          ## Don't reply to register request
          ## TODO: register API with replies
          case register(id, route, metadata, state) do
            {:ok, state} ->
              {:noreply, state}
            other ->
              other
          end

        other ->
          Logger.warn(
            "Unable to parse message payload: #{inspect(message)} reason: #{inspect(other)}"
          )

          {:noreply, state}
      end

    reply(result, message)
  end

  def with_storage(state, fun) do
    {storage_mod, storage_state} = Map.get(state, :storage)
    {result, new_storage_state} = fun.(storage_mod, storage_state)
    {result, Map.put(state, :storage, {storage_mod, new_storage_state})}
  end

  def list(state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.list(storage_state)
    end)
  end

  def get(id, state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.get(id, storage_state)
    end)
  end

  def register(id, route, metadata, state) do
    with_storage(state, fn storage_mod, storage_state ->
      storage_mod.register(id, route, metadata, storage_state)
    end)
  end

  def parse_request(message) do
    payload = Message.payload(message)

    case payload do
      <<0>> <> request_v0 ->
        ## TODO: better way to encode request data??
        case BareExtended.decode(request_v0, request_schema()) do
          {:ok, {:list, ""}} ->
            :list
          {:ok, {:get, id}} ->
            {:get, id}

          {:ok, {:register, %{id: id, metadata: metadata}}} ->
            ## Using message return route as a route in register request.
            ## TODO: remove route from request to only use the traced return route?
            {:register, id, Message.return_route(message), metadata}

          other ->
            other
        end
      other ->
        {:error, {:invalid_request_version, other}}
    end
  end

  def reply({:noreply, state}, _message) do
    {:ok, state}
  end

  def reply({reply, state}, message) do
    Router.route(Message.reply(message, state.address, format_reply(reply)))
    {:ok, state}
  end

  def format_reply(reply) do
    ## TODO: maybe use better distinction between results (request id/function?)
    formatted = case reply do
      {:ok, service_info} ->
        :bare.encode(service_info, service_info_schema())

      [] ->
        :bare.encode([], {:array, service_info_schema()})
      [%ServiceInfo{} | _] = list ->
        :bare.encode(list, {:array, service_info_schema()})

      :ok ->
        ## TODO: meaningful response for registration
        ""

      {:error, _reason} ->
        ## TODO: error encoding
        ""
    end
    <<0>> <> formatted
  end

  ## BARE schemas

  def request_schema() do
   [
     list: {:data, 0},
     get: :string,
     register: {:struct, [id: :string, metadata: {:map, :string, :data}]}
   ]
  end

  def register_schema() do
    {:struct, [id: :string, metadata: {:map, :string, :data}]}
  end

  def encode_register_request(id, metadata) do
    <<0>> <> BareExtended.encode({:register, %{id: id, metadata: metadata}}, request_schema())
  end

  ## To be used with this schema, routes should be normalized to (type, value) maps
  ## TODO: improve encode/decode logic to work with other address formats
  def service_info_schema() do
    {:struct,
     [
       id: :string,
       route: Ockam.Wire.Binary.V2.bare_spec(:route),
       metadata: {:map, :string, :data}
     ]}
  end
end

defmodule Ockam.Hub.Service.Discovery.Storage do
  @moduledoc """
  Storage module behaviour for discovery service
  """
  alias Ockam.Hub.Service.Discovery.ServiceInfo

  @type storage_state() :: any()
  @type metadata() :: %{binary() => binary()}

  @callback init() :: storage_state()
  @callback list(storage_state()) :: [ServiceInfo.t()]
  @callback get(id :: binary(), storage_state()) :: {:ok, ServiceInfo.t()} | {:error, :not_found}
  @callback register(id :: binary(), route :: [Ockam.Address.t()], metadata(), storage_state()) ::
              :ok | {:error, reason :: any()}
end

defmodule Ockam.Hub.Service.Discovery.Storage.Memory do
  @moduledoc """
  In-memory storage for discovery service.
  Stores registered workers as a map of %{id => ServiceInfo}
  """
  @behaviour Ockam.Hub.Service.Discovery.Storage

  alias Ockam.Hub.Service.Discovery.ServiceInfo

  @type storage_state() :: %{binary() => ServiceInfo.t()}

  def init() do
    %{}
  end

  def get(id, state) do
    case Map.fetch(state, id) do
      {:ok, result} -> {{:ok, result}, state}
      :error -> {{:error, :not_found}, state}
    end
  end

  def list(state) do
    {Map.values(state), state}
  end

  def register(id, route, metadata, state) do
    ## TODO: option to override or ignore?
    new_state = Map.put(state, id, %ServiceInfo{id: id, route: route, metadata: metadata})
    {:ok, new_state}
  end
end
