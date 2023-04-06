# local_wallet_relay
local relay server between desktop app (games) and  browser extension wallet.

(payload for contract) desktop app -> local relay -> browser -> wallet (signed transaction)  

(view etc...) desktop app <- local relay <- browser <- wallet (address)

motivation
1. desktop apps need NOT to know private keys.
1. user can use their wallet and account.
1. simplify(huge !!) desktop app codes.
