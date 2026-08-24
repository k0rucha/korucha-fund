-- Canonicalize values accepted by older versions of the admin form.
UPDATE transactions
SET symbol = UPPER(TRIM(symbol)),
    txn_type = UPPER(TRIM(txn_type)),
    currency = UPPER(TRIM(currency)),
    fx_rate_to_jpy = CASE WHEN UPPER(TRIM(currency)) = 'JPY' THEN NULL ELSE fx_rate_to_jpy END,
    notes = NULLIF(TRIM(notes), '');

-- Defense in depth for any future write path that bypasses domain validation.
CREATE TRIGGER validate_transaction_before_insert
BEFORE INSERT ON transactions
BEGIN
  SELECT CASE
    WHEN NEW.symbol = ''
      OR LENGTH(NEW.symbol) > 32
      OR NEW.symbol GLOB '*[^A-Z0-9._^=-]*'
      THEN RAISE(ABORT, 'invalid transaction symbol')
    WHEN NEW.txn_type NOT IN ('BUY', 'SELL')
      THEN RAISE(ABORT, 'invalid transaction type')
    WHEN NEW.quantity <= 0
      THEN RAISE(ABORT, 'invalid transaction quantity')
    WHEN NEW.price <= 0
      THEN RAISE(ABORT, 'invalid transaction price')
    WHEN NEW.fee < 0
      THEN RAISE(ABORT, 'invalid transaction fee')
    WHEN NEW.currency NOT IN ('JPY', 'USD')
      THEN RAISE(ABORT, 'invalid transaction currency')
    WHEN NEW.currency = 'USD' AND (NEW.fx_rate_to_jpy IS NULL OR NEW.fx_rate_to_jpy <= 0)
      THEN RAISE(ABORT, 'invalid transaction FX rate')
    WHEN NEW.currency = 'JPY' AND NEW.fx_rate_to_jpy IS NOT NULL
      THEN RAISE(ABORT, 'JPY transaction must not have an FX rate')
    WHEN LENGTH(COALESCE(NEW.notes, '')) > 1000
      THEN RAISE(ABORT, 'transaction notes too long')
  END;
END;
