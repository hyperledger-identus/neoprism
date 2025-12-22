## Objective

We want to include the `tx` in `BlockMetadata` and index them in the database.
The `tx_hash` is the transaction hash from the cardano blockchain which `DltSource` should provide.
The implementation of the `DltSource` should have this for both oura and dbsync.

## Constraints

- The transaction id already has the newtype `TxId`. Use that.
- Do not worry about old indexed data where the tx_hash will be null. We will nuke old data and re-index everything.

## Resources

- For oura implementation `EventContext` please check `https://github.com/patextreme/oura/blob/61dc55e7cb580af1f6b37a30b69146d614c6214d/src/framework/legacy_v1.rs#L199`

