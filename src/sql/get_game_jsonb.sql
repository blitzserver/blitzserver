SELECT 
  jsonb_build_object(
    'name', g.name,
    'id', g.id,
    'next_turn', g.next_turn,
    'next_turn_unix', ROUND(EXTRACT(epoch FROM g.next_turn)),
    'current_turn', (
      SELECT MAX(t.turn_number)
      FROM turns t
      WHERE t.game_id = g.id AND t.archived = false
    ),
    'players', (
      SELECT jsonb_object_agg(
        p.nationid,
        jsonb_build_object(
          'name', p.name,
          'nation_name', n.name,
          'current_turn_status', (
            SELECT pt.status
            FROM player_turns pt
            JOIN (
              SELECT MAX(turn_number) as max_turn
              FROM turns
              WHERE game_id = g.id AND archived = false
            ) current_turn ON pt.turn_number = current_turn.max_turn
            WHERE pt.game_id = g.id
              AND pt.nation_id = p.nationid
          )
        )
      )
      FROM players p
      JOIN nations n ON p.nationid = n.nation_id AND p.game_id = n.game_id
      WHERE p.game_id = g.id
    ),
    'turns', (
      SELECT jsonb_object_agg(
        t.turn_number,
        (
          SELECT jsonb_object_agg(
            pt.nation_id,
            pt.status
          )
          FROM player_turns pt
          WHERE pt.game_id = t.game_id 
            AND pt.turn_number = t.turn_number
        )
      )
      FROM turns t
      WHERE t.game_id = g.id
        AND t.archived = false
    ),
    'port', g.port
  ) AS "details!"
FROM games g
WHERE g.id=$1;

