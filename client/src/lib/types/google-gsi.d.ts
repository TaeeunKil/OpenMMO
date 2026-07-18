// Minimal typings for the Google Identity Services script we load at runtime.
interface GoogleCredentialResponse {
  credential: string
}

interface GoogleIdApi {
  initialize(config: {
    client_id: string
    callback: (response: GoogleCredentialResponse) => void
    auto_select?: boolean
    itp_support?: boolean
    use_fedcm_for_prompt?: boolean
  }): void
  prompt(): void
  renderButton(
    parent: HTMLElement,
    options: {
      theme?: 'outline' | 'filled_blue' | 'filled_black'
      size?: 'large' | 'medium' | 'small'
      text?: 'signin_with' | 'signup_with' | 'continue_with' | 'signin'
      shape?: 'rectangular' | 'pill' | 'circle' | 'square'
      width?: number
    }
  ): void
}

interface Window {
  google?: {
    accounts: {
      id: GoogleIdApi
    }
  }
}
