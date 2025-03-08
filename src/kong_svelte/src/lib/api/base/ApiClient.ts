import { API_URL } from "$lib/api/index";
import { browser } from "$app/environment";

/**
 * Base API client with common functionality for all API clients
 */
export class ApiClient {
  protected baseUrl: string;

  constructor(baseUrl: string = API_URL) {
    this.baseUrl = baseUrl || "";
  }

  /**
   * Creates default request options with proper headers
   */
  protected createRequestOptions(method: string, additionalHeaders: Record<string, string> = {}): RequestInit {
    return {
      method,
      headers: {
        'Content-Type': 'application/json',
        'Cache-Control': 'no-cache, no-store, must-revalidate',
        'Pragma': 'no-cache',
        ...additionalHeaders
      },
      cache: 'no-store'
    };
  }

  /**
   * Performs a GET request
   */
  public async get<T>(
    endpoint: string, 
    params: Record<string, string> = {}, 
    additionalHeaders: Record<string, string> = {}
  ): Promise<T> {
    // Check if we're in a browser environment
    if (!browser) {
      throw new Error("API calls can only be made in the browser");
    }
    
    // Build query string
    const queryString = new URLSearchParams(params).toString();
    const url = `${this.baseUrl}${endpoint}${queryString ? `?${queryString}` : ''}`;
    
    // Create request options
    const options = this.createRequestOptions('GET', additionalHeaders);
    
    // Make the request
    return this.fetchWithErrorHandling<T>(url, options);
  }

  /**
   * Performs a POST request
   */
  public async post<T>(
    endpoint: string, 
    body: any, 
    additionalHeaders: Record<string, string> = {}
  ): Promise<T> {
    // Check if we're in a browser environment
    if (!browser) {
      throw new Error("API calls can only be made in the browser");
    }
    
    const url = `${this.baseUrl}${endpoint}`;
    
    // Create request options
    const options = this.createRequestOptions('POST', additionalHeaders);
    options.body = JSON.stringify(body);
    
    // Make the request
    return this.fetchWithErrorHandling<T>(url, options);
  }

  /**
   * Performs a PUT request
   */
  public async put<T>(
    endpoint: string, 
    body: any, 
    additionalHeaders: Record<string, string> = {}
  ): Promise<T> {
    // Check if we're in a browser environment
    if (!browser) {
      throw new Error("API calls can only be made in the browser");
    }
    
    const url = `${this.baseUrl}${endpoint}`;
    
    // Create request options
    const options = this.createRequestOptions('PUT', additionalHeaders);
    options.body = JSON.stringify(body);
    
    // Make the request
    return this.fetchWithErrorHandling<T>(url, options);
  }

  /**
   * Performs a DELETE request
   */
  public async delete<T>(
    endpoint: string, 
    additionalHeaders: Record<string, string> = {}
  ): Promise<T> {
    // Check if we're in a browser environment
    if (!browser) {
      throw new Error("API calls can only be made in the browser");
    }
    
    const url = `${this.baseUrl}${endpoint}`;
    
    // Create request options
    const options = this.createRequestOptions('DELETE', additionalHeaders);
    
    // Make the request
    return this.fetchWithErrorHandling<T>(url, options);
  }

  /**
   * Fetches with error handling
   */
  private async fetchWithErrorHandling<T>(url: string, options: RequestInit): Promise<T> {
    try {
      const response = await fetch(url, options);
      
      // Handle non-OK responses
      if (!response.ok) {
        let errorMessage: string;
        
        try {
          // Try to parse error response as JSON
          const errorData = await response.json();
          errorMessage = errorData.message || `HTTP error ${response.status}: ${response.statusText}`;
        } catch (e) {
          // If parsing fails, use status text
          errorMessage = `HTTP error ${response.status}: ${response.statusText}`;
        }
        
        throw new Error(errorMessage);
      }
      
      // Handle empty responses
      const text = await response.text();
      if (!text.trim()) {
        return {} as T;
      }
      
      // Parse JSON response
      return JSON.parse(text) as T;
    } catch (error) {
      console.error('API request failed:', error);
      throw error;
    }
  }
} 