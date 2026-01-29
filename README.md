# ruft
A CE-style Parallel Raft variant with explicit confirmed/committed separation and ordered apply guarantees.

parallel raft paper: https://github.com/HappyCS-Gu/Parallel-Raft-tla/blob/master/doc/2020.8-jos.pdf

# 选举
Leader:
1. 节点在 election timeout 到期后进入 Candidate
2. term += 1, 记录 voted_for = self
3. 向所有 voting 节点发送：RequestVote{ term, candidate_id, last_log_index, last_log_term }

Follower: 
1. 若 req.term < current_term → 拒绝
2. 若 req.term > current_term:
   - current_term = req.term
   - voted_for = None
   - 切换为 Follower
3. 若 voted_for != None 且 voted_for != candidate → 拒绝
4. req.last_log_term < local_last_log_term → 拒绝
5. req.last_log_term == local_last_log_term && req.last_log_index < local_last_log_index → 拒绝
6. 投票并记录 voted_for = candidate_id

Candidate 成为 Leader:
1. 收到 多数派投票
2. 切换为 Leader
3. 初始化所有 Follower
   - inflight = {}
   - confirmed = {}
   - committed_index = local_committed

# Parallel Append
Leader 发起 AppendEntries（并行）：
1. Leader 可同时选择多个未复制日志 entry
2. 对每个 follower 并行发送：
   - term
   - leader_id
   - entry_id
   - log_index
   - log_term
   - prev_log_index // 表示 entry 的逻辑前驱，用于最终 commit 顺序， 而不是要求 follower 当前必须已存在. prev_log_index ≡ entry_id - 1
   - prev_log_term
   - entry_data
   - leader_confirmed_index
     
Follower 处理 AppendEntries:
1. 若 req.term < current_term → reject
2. 若 req.term > current_term
    - 更新 term
    - 切换为 follower
3. 若 prev_log_index 不存在
    - ❌ 不立即拒绝整个请求
    - 标记该 entry 为 pending，写入 pending_logs
4. 若 prev_log_index 不存在 && term 不匹配
    - 删除 main_logs 该 index 及之后日志
5. 若 prev_log_index 存在 且 term 匹配
    - 将 entry 写入 main_log
6. 回复 Leader
    - term
    - entry_id,
    - log_index
    - accepted = true | false // true ≠ 可提交, 仅仅代表follower 对 entry 已持久化（pending 或 main_log）

Leader 处理 AppendEntriesResp
1. 若 resp.term > current_term
    - 立刻降级为 follower
2. 根据 entry_id 精确定位 inflight
3. 若 accepted
   - 标记该 follower confirmed(entry)
4. 若 rejected
   - 仅回滚该 entry, 不影响其他 inflight

# Merge & Commit
1. Entry Confirmed 规则（Parallel 新增）
    - confirmed(i) = 存在多数派 follower accepted(i)
    - confirmed ≠ committed
2. Commit 推进规则（顺序保证）: Leader 周期性执行：
    - 找到最大的 i，满足：confirmed(i) = true && term(i) == current_term
    - 所有小于 i 的 entry，都已经 commit
    - commit(i)，推进 commit_index
3. Follower 提交与 apply
    - follower维护pending_logs, committed_index
    - 收到 leader 的 leader_confirmed_index 且本地日志连续，则顺序commit

# 冲突 / 回退 / 重同步规则
1. 单 entry 冲突
   - 仅回滚冲突 entry
   - 其他 inflight 不受影响
2. Follower 落后严重
   - Leader 发现：大量 entry rejected
   - 切换该 follower 为：catch_up_mode
   - 回到 串行 AppendEntries
   - 对齐后再恢复并行
