# CyberOrto

![photo of CyberOrto](cyberorto%20foto.jpg)

## Comandi comodi per fare richieste all'orchestrator

Per ottenere lo stato (il link funziona anche da browser):

```sh
curl http://127.0.0.1:8000/state
```

CommandListAction che contiene un singolo comando che fa muovere l'orto:

```sh
curl http://127.0.0.1:8000/queue/add_action_list --request POST --header 'Content-Type: application/json' --data '[{"Move": {"x": 1, "y": 0.5, "z": -0.6}}]'
```

CommandListAction supporta vari altri comandi, vedere [command_list.rs](./orchestrator/src/action/command_list.rs)! Ad esempio:

```sh
curl http://127.0.0.1:8000/queue/add_action_list --request POST --header 'Content-Type: application/json' --data '["ToggleLed", {"WaterCooldown": {"secs": 5,"nanos": 0}}, "Reset"]'
```

Per killare l'azione in esecuzione al momento (mette sempre in pausa l'esecuzione anche se success=false, interrompe anche dei passi delle azioni a metà):

```sh
curl http://127.0.0.1:8000/queue/kill_running_action --request POST --header 'Content-Type: application/json' --data '{"action_id": 96, "keep_in_queue": false}'
```

Per mettere e togliere dalla pausa (la pausa entra in vigore dopo che il passo dell'azione in esecuzione al momento finisce):

```sh
curl http://127.0.0.1:8000/queue/pause --request POST
curl http://127.0.0.1:8000/queue/unpause --request POST
```

Per svuotare la coda di azioni (lasciando però che l'azione corrente finisca il passo in esecuzione al momento):

```sh
curl http://127.0.0.1:8000/queue/clear --request POST
```
