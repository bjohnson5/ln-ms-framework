// External Modules
use serde::{Serialize, Deserialize};

/*
 * This struct represents a transaction in the simulation
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimTransaction {
    pub id: Option<String>, // set to none until the transaction is sent by the node and then it is assigned a value
    pub src_node: String, // the node that is sending the payment
    pub dest_node: String, // the node that is receiving the payment
    pub amount_sats: u64, // amount in sats
    pub status: SimTransactionStatus // set to none until the transaction is sent
}

/*
 * The status of a transaction.
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SimTransactionStatus {
    NONE, // the transaction has been created
    PENDING, // the transaction  has been initiated
    SUCCESSFUL, // the transaction was received and successful
    FAILED // the transaction failed
}