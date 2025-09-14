# CyberOrto

![photo of CyberOrto](cyberorto%20foto.jpg)

## Eseguire l'orchestrator

Per far partire l'orchestrator (che espone anche un server) usare questo comando

```sh
cargo run -- --ports=PORTS
```

Per eseguirlo sul raspberry con i file `.rs` usati in locale, usare questo:

```sh
scp -r ./embedcore/src mindshub@192.168.1.102:/home/mindshub/Desktop/cyberorto/embedcore && scp -r ./orchestrator/src mindshub@192.168.1.102:/home/mindshub/Desktop/cyberorto/orchestrator && ssh -t mindshub@192.168.1.102 "bash -l -c 'cd /home/mindshub/Desktop/cyberorto/orchestrator; ROCKET_ADDRESS=0.0.0.0 cargo run -- --ports=autosimulated'"
```

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

## Se i motori funzionano solo quando il debugger è attaccato

Disattivare la feature "defmt" quando si compilano i motori, altrimenti quando i motori provano a inviare qualcosa al debugger (tipo `info!()`) si blocca tutto.

## Altre cose utili

Da `./stepper-ch32v305`, eseguire questo per controllare se compilano tutti i bin.

```sh
for i in ./src/bin/*; do cargo build --release --bin "${$(basename "$i")%.*}"; done
```
