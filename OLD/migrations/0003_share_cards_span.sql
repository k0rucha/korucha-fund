-- 共有カードに発行時のチャート期間を保存する
ALTER TABLE share_cards ADD COLUMN default_span TEXT NOT NULL DEFAULT 'all';
