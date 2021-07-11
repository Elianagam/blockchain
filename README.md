Blockchain Rustica
==================


### Implementacion de la blockchain:

Caracteristicas basicas de una blockchain:

 - Cada uno de los nodos tiene una copia local de la blockchain
 - Se broadcastean los nuevos bloques que entran en la blockchain
 - En el mundo real para ingresar un bloque nuevo hay que hacer mining (proceso de encontrar un hash que tenga relacion con el nonce del bloque)

Nuestra implementacion: 
    
 - Cada nodo tiene una referencia a su precedente (hash)
 - Cada nodo tiene una copia local de la blockchain
 - Cuando algun nodo quiere ingresar un nuevo valor este hashea el bloque y lo agrega a la blockchain (aca usamos el algoritmo de concurrencia distribuida)
 - Ademas envia al nodo lider un mensaje para que broadcastee el nodo al resto (algoritmo de eleccion de lider)
 - Cada nodo actualiza su copia local de la blockchain

