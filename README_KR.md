# HOMEI Jailbreak

**독립 iOS 보안 및 탈옥 프레임워크** · 코드명 HOMEI-A17

> **HOMEI는 연구 단계 프레임워크입니다. 작동하는 탈옥이 아니며, 익스플로잇을 포함하지 않고, 어떤 기기에도 연결하거나 수정하지 않습니다.** 어떤 기기에 대해서도 지원을 주장하지 않습니다. 아래 연구 대상의 호환성은 **확립되지 않음(NOT ESTABLISHED)** 입니다.

[English](README.md)

## 1. 상태

| 항목 | 상태 |
|---|---|
| 버전 | 0.1.0 (초기 기반, 미출시) |
| 대상 호환성 | **확립되지 않음** |
| 준비 수준 | LEVEL 0 / **NO-GO** |
| 탈옥 백엔드 | 없음 |
| Rust 코어, 카탈로그, 부트스트랩 시뮬레이션, CLI | 호스트에서 빌드 및 테스트 완료 (테스트 56개) |
| Swift 패키지 및 SwiftUI 앱 | 작성됨, **컴파일하지 않음** (개발 환경에 Swift와 Xcode 없음) |
| iOS 바이너리 | **빌드하지 않음** |

## 2. HOMEI란

증거 기반 호환성 엔진, 공개 연구 트래커와 연결된 오프라인 연구 색인, 닫힌 허용 목록 방식의 안전 관리자, 메타데이터 전용 패키지 카탈로그, 부트스트랩 계획 시뮬레이터, 복구 문서, 영어와 한국어를 지원하는 독창적인 SwiftUI 인터페이스로 구성됩니다. 독립적으로 개발되었으며 Dopamine, Relaxin, unc0ver 또는 다른 탈옥의 포크가 아니고 그들로부터 아무것도 복사하지 않았습니다.

## 3. HOMEI가 아닌 것

익스플로잇, 커널 또는 SPTM 우회, 실제로 설치하는 부트스트랩, 실제로 설치하는 패키지 관리자, 기기 도구가 아닙니다. USB, 페어링, 탐색, 폴링 코드가 없습니다. 권한이 필요한 기능은 가짜 성공이 아니라 명시적인 '지원 불가' 결과를 반환합니다. Apple 또는 어떤 탈옥 프로젝트와도 제휴하거나 승인을 받지 않았습니다.

## 4. 연구 대상

| 항목 | 값 |
|---|---|
| 기기 | iPhone 15 Pro Max (`iPhone16,2`) |
| SoC | Apple A17 Pro |
| 펌웨어 | iOS 27.0.1, 빌드 24A446 |
| 아키텍처 | arm64e |

대상은 공개된 연구 프로필입니다. 다른 기기, 버전, 칩의 결과는 이 대상에 적용되지 않습니다. 보안 관련 결과는 연구 트래커에서 가져오며 트래커가 기준 자료입니다: [ios27-a17pro-jailbreak-tracker](https://github.com/PalanBanana/ios27-a17pro-jailbreak-tracker).

## 5. 아키텍처

모듈: 기기 프로필, 호환성 엔진, 증거 데이터베이스, 연구 인텔리전스, 안전 관리자, 진단, 패키지 카탈로그, 부트스트랩 계획, 복구, 권한 기능 상태. 이식 가능한 코어는 Rust, 검증은 Python과 JSON Schema, 인터페이스는 Foundation만 쓰는 키트 위의 SwiftUI입니다. [Docs/ARCHITECTURE.md](Docs/ARCHITECTURE.md)를 참조하세요.

## 6. 빌드와 테스트

```
cargo test --workspace --locked
python3 -m pip install jsonschema && python3 scripts/validate_data.py
cargo run -p homei-cli -- evaluate FIX-A
```

Swift 소스와 iOS 앱은 Xcode가 있는 macOS가 필요하며 아직 빌드되지 않았습니다. [Docs/DEVELOPMENT.md](Docs/DEVELOPMENT.md)를 참조하세요.

## 7. 안전

애플리케이션은 보기, 검증, 정리된 보고서 내보내기, 메타데이터 탐색, 시뮬레이션만 수행할 수 있습니다. 그 밖의 모든 것은 거부됩니다. 제품 코드에 네트워크, 프로세스, 기기, 파일 쓰기 구문이 추가되면 소스 위생 테스트가 빌드를 실패시킵니다. [Docs/SAFETY.md](Docs/SAFETY.md)와 [Docs/RECOVERY.md](Docs/RECOVERY.md)를 참조하세요. 일상적으로 쓰는 기기를 테스트 대상으로 사용하지 마세요.

## 8. 참고 프로젝트

Dopamine, Relaxin, unc0ver는 저장소 메타데이터, README, 디렉터리 이름만으로 높은 수준의 제품 관행을 살펴보았으며 코드나 자산은 사용하지 않았습니다. 세 참고 프로젝트 중 둘은 예상과 달랐습니다. Relaxin 저장소는 소스가 없는 비공식 다운로드 미러이고, `Unc0ver-inc/Unc0ver`는 원본이 아닌 유사 저장소입니다. [Docs/REFERENCE_PROJECT_AUDIT.md](Docs/REFERENCE_PROJECT_AUDIT.md)와 [Docs/PRODUCT_DIFFERENTIATION.md](Docs/PRODUCT_DIFFERENTIATION.md)를 참조하세요.

## 9. 문서

[제품 요구사항](Docs/PRODUCT_REQUIREMENTS.md) · [아키텍처](Docs/ARCHITECTURE.md) · [기기 호환성](Docs/DEVICE_COMPATIBILITY.md) · [안전](Docs/SAFETY.md) · [복구](Docs/RECOVERY.md) · [디자인 명세](Docs/DESIGN_SPEC.md) · [개발](Docs/DEVELOPMENT.md) · [로드맵](Docs/ROADMAP.md) · [서드파티 고지](THIRD_PARTY_NOTICES.md) · [기여](CONTRIBUTING.md) · [보안](SECURITY.md)

## 10. 라이선스

코드: MIT ([LICENSE](LICENSE)). 문서: CC BY 4.0 ([LICENSE-DOCS](LICENSE-DOCS)). Apple, iPhone, iOS는 Apple Inc.의 상표입니다.
