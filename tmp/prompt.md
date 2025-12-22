## Objective

Now that we have indexed the `tx_hash` in the database, we want to expose the API for user to query transaction centric data.
You will have to create a new endpoint `GET /api/transactions/{tx_id}` to let user query the transaction data.
The transaction data should contain the transaction detail and the operations inside it.
You only need to provide the endpoit to get transaction by ID, no need to do transaction listing.

## Hint

You'll have to implement a new method `get_raw_operations_by_tx_id` in the `RawOperationRepo` then use this as a data source in the new API endpoint.
You can use the view `raw_operation_by_did` which has the `tx_hash` field.

