# Operational & Security Policies

이 문서는 전체 ZAPPY 시스템이 반드시 준수해야 하는 정책을 명시한다. 모든 런타임·오케스트레이터 코드는 부팅 시 본 정책을 로드하고, 위반 시 즉시 self_upgrade 루프 또는 운영자 알림을 트리거해야 한다.

## 1. Access Control
- 모든 외부 정책/보안 룰은 서명된 JSON 또는 S3/KV 스토리지에서만 수신한다.
- 민감 명령(자율 루프 실행, self-upgrade 등)은 멀티 팩터 승인 또는 내부 정책 토큰으로 승인된 세션에서만 실행한다.

## 2. Data Governance
- 로그 및 텔레메트리는 `logs/` 이하 구조를 사용하며, 30일 이상 장기 저장 시 암호화된 보관소로 이동한다.
- 외부 데이터 파이프라인과 결합할 때는 스키마를 `docs/`에 사전 정의하고, 실 데이터는 익명화 또는 최소화된 형태로만 사용한다.

## 3. Incident Response
- 실패·편향 감지 시 self_upgrade 루프를 자동 기동하고, `POLICY-IR` 태그로 이벤트 버스에 게시한다.
- 정책 위반 이벤트는 5분 내 운영자 채널로 전파해야 하며, 대응 상태는 knowledge 모듈에 기록한다.

## 4. Change Management
- 모든 런타임은 오케스트레이터 지시 없이는 임의 변경을 금지하며, 오케스트레이터는 정책 버전(`POLICIES.md` 해시)과 함께 명령을 기록한다.
- 외부 정책 파일이 갱신되면 즉시 actions/autonomy/planning 모듈에 브로드캐스트 후 재검증을 수행한다.

## 5. Observability & Audit
- 주요 모듈(actions/autonomy/planning/reasoning/world/simulation/self-upgrade)을 거치는 모든 명령은 ExperienceHub 파이프라인에 남겨 재학습에 활용한다.
- 감사 추적은 삭제할 수 없으며, 운영자만이 아카이브 롤오버를 승인할 수 있다.

**현재 정책 준수 기술은 단순 참고 수준이며, 상용화 실행 시 추가적 보완이 필요함**
