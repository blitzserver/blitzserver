-- Simplified version using basic JSON
SELECT
  json_object(
    'name', g.name,
    'id', g.id,
    'next_turn', datetime(g.next_turn),
    'next_turn_unix', CAST(strftime('%s', g.next_turn) AS INTEGER),
    'current_turn', COALESCE((
      SELECT MAX(t.turn_number)
      FROM turns t
      WHERE t.game_id = g.id AND t.archived = 0
    ), 0),
    'port', g.port
  ) "details"
FROM games g
WHERE g.id=$1;
