using System;
using System.Collections;
using System.Collections.Generic;
using SpacetimeDB;
using SpacetimeDB.Types;
using TMPro;
using UnityEngine;
using UnityEngine.UI;

public class GameManager : MonoBehaviour
{
    const string SERVER_URL = "http://127.0.0.1:3000";
    const string MODULE_NAME = "blackholio";

    public static event Action OnConnected;
    public static event Action OnSubscriptionApplied;

    public float borderThickness = 2;
    public Material borderMaterial;
    public TMP_InputField accountInput;
    public Button logoutBtn;

	public static GameManager Instance { get; private set; }
    public static Identity LocalIdentity { get; private set; }
    public static DbConnection Conn { get; private set; }

    public static Dictionary<uint, EntityController> Entities = new Dictionary<uint, EntityController>();
    public static Dictionary<uint, PlayerController> Players = new Dictionary<uint, PlayerController>();

    public void Start()
    {
        var account = accountInput.text;
        
        Instance = this;
        Application.targetFrameRate = 60;
        Application.runInBackground = true;

        // In order to build a connection to SpacetimeDB we need to register
        // our callbacks and specify a SpacetimeDB server URI and module name.
        var builder = DbConnection.Builder()
            .OnConnect(HandleConnect)
            .OnConnectError(HandleConnectError)
            .OnDisconnect(HandleDisconnect)
            .WithUri(SERVER_URL)
            .WithModuleName(MODULE_NAME);

        // If the user has a SpacetimeDB auth token stored in the Unity PlayerPrefs,
        // we can use it to authenticate the connection.
        if (AuthToken.Token != "")
        {
            builder = builder.WithToken(AuthToken.Token);
        }

        // Building the connection will establish a connection to the SpacetimeDB
        // server.
        Conn = builder.Build();
    }

    // Called when we connect to SpacetimeDB and receive our client identity
    void HandleConnect(DbConnection conn, Identity identity, string token)
    {
        Debug.Log("Connected.");
        Debug.Log($"identity: {identity}, token: {token}");
        AuthToken.SaveToken(token);
        LocalIdentity = identity;
        
        accountInput.gameObject.SetActive(false);
        logoutBtn.gameObject.SetActive(true);

        conn.Db.Circle.OnInsert += CircleOnInsert;
        conn.Db.Entity.OnUpdate += EntityOnUpdate;
        conn.Db.Entity.OnDelete += EntityOnDelete;
        conn.Db.Food.OnInsert += FoodOnInsert;
        conn.Db.Player.OnInsert += PlayerOnInsert;
        conn.Db.Player.OnDelete += PlayerOnDelete;

        OnConnected?.Invoke();

        // Request all tables
        Conn.SubscriptionBuilder()
            .OnApplied(HandleSubscriptionApplied)
            .SubscribeToAllTables();
    }

    void HandleConnectError(Exception ex)
    {
        Debug.LogError($"Connection error: {ex}");
    }

    void HandleDisconnect(DbConnection _conn, Exception ex)
    {
        Debug.Log("Disconnected.");
        if (ex != null)
        {
            Debug.LogException(ex);
        }
        
        accountInput.gameObject.SetActive(true);
        logoutBtn.gameObject.SetActive(false);
    }

    private void HandleSubscriptionApplied(SubscriptionEventContext ctx)
    {
        Debug.Log("Subscription applied!");
        OnSubscriptionApplied?.Invoke();

        // Once we have the initial subscription sync'd to the client cache
        // Get the world size from the config table and set up the arena
        var worldSize = Conn.Db.Config.Id.Find(0).WorldSize;
        SetupArena(worldSize);

        // Call enter game with the player name 3Blave
        ctx.Reducers.EnterGame("3Blave");
    }

    public static bool IsConnected()
    {
        return Conn != null && Conn.IsActive;
    }

    public void Disconnect()
    {
        Conn.Disconnect();
        Conn = null;
    }
    
    private void SetupArena(float worldSize)
    {
        CreateBorderCube(new Vector2(worldSize / 2.0f, worldSize + borderThickness / 2),
            new Vector2(worldSize + borderThickness * 2.0f, borderThickness)); //North
        CreateBorderCube(new Vector2(worldSize / 2.0f, -borderThickness / 2),
            new Vector2(worldSize + borderThickness * 2.0f, borderThickness)); //South
        CreateBorderCube(new Vector2(worldSize + borderThickness / 2, worldSize / 2.0f),
            new Vector2(borderThickness, worldSize + borderThickness * 2.0f)); //East
        CreateBorderCube(new Vector2(-borderThickness / 2, worldSize / 2.0f),
            new Vector2(borderThickness, worldSize + borderThickness * 2.0f)); //West

        // Set the world size for the camera controller
        CameraController.WorldSize = worldSize;
    }

    private void CreateBorderCube(Vector2 position, Vector2 scale)
    {
        var cube = GameObject.CreatePrimitive(PrimitiveType.Cube);
        cube.name = "Border";
        cube.transform.localScale = new Vector3(scale.x, scale.y, 1);
        cube.transform.position = new Vector3(position.x, position.y, 1);
        cube.GetComponent<MeshRenderer>().material = borderMaterial;
    }
    
    private static void CircleOnInsert(EventContext context, Circle insertedValue)
    {
        var player = GetOrCreatePlayer(insertedValue.PlayerId);
        var entityController = PrefabManager.SpawnCircle(insertedValue, player);
        Entities.Add(insertedValue.EntityId, entityController);
    }

    private static void EntityOnUpdate(EventContext context, Entity oldEntity, Entity newEntity)
    {
        if (!Entities.TryGetValue(newEntity.EntityId, out var entityController))
        {
            return;
        }
        entityController.OnEntityUpdated(newEntity);
    }

    private static void EntityOnDelete(EventContext context, Entity oldEntity)
    {
        if (Entities.Remove(oldEntity.EntityId, out var entityController))
        {
            entityController.OnDelete(context);
        }
    }

    private static void FoodOnInsert(EventContext context, Food insertedValue)
    {
        var entityController = PrefabManager.SpawnFood(insertedValue);
        Entities.Add(insertedValue.EntityId, entityController);
    }

    private static void PlayerOnInsert(EventContext context, Player insertedPlayer)
    {
        GetOrCreatePlayer(insertedPlayer.PlayerId);
    }

    private static void PlayerOnDelete(EventContext context, Player deletedvalue)
    {
        if (Players.Remove(deletedvalue.PlayerId, out var playerController))
        {
            GameObject.Destroy(playerController.gameObject);
        }
    }

    private static PlayerController GetOrCreatePlayer(uint playerId)
    {
        if (!Players.TryGetValue(playerId, out var playerController))
        {
            var player = Conn.Db.Player.PlayerId.Find(playerId);
            playerController = PrefabManager.SpawnPlayer(player);
            Players.Add(playerId, playerController);
        }

        return playerController;
    }
}