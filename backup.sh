#!/bin/bash
export PGPASSWORD="password"
BACKUP_FILE="/backup/db_backup_$(date +%Y%m%d%H%M%S).sql"
mkdir -p /backup
pg_dump -U aot -h db aot > "$BACKUP_FILE" 2>> /backup/backup.log
echo "Backup completed at $(date)" >> /backup/backup.log