//
//  OnboardingView.swift
//  App
//
//  Created by Juan Pablo Gutierrez on 25/05/23.
//
//  Represents the onboarding screen

import SwiftUI

//Sets burrow visited status
func setVisited() {
    UserDefaults.standard.set(true, forKey: "launchedBefore")
    NSApp.windows.first?.close()
}

@available(macOS 13.0, *)
struct OnboardingView: View {
    var body: some View {
        ZStack(alignment: .center) {
            Image("OnboardingBackground")
                .resizable(resizingMode: .stretch)
                .aspectRatio(contentMode: .fit)
                .scaledToFill()
            Color.black
                .opacity(0.6)
                .cornerRadius(15)
                .blur(radius: 0.2)
                .edgesIgnoringSafeArea(.all)
                .frame(width: 450, height: 300)
            VStack(alignment: .center) {
                Text("Welcome to burrow").font(.system(size: 24, weight: .bold, design: .rounded))
                Spacer().frame(height: /*@START_MENU_TOKEN@*/20.0/*@END_MENU_TOKEN@*/)
                Text("It is a best-in-class tool for burrowing through firewalls.").font(.system(size: 14))
                Spacer().frame(height: 10.0)
                Text("Built by teenagers at HackClub").font(.system(size: 14))
                Button(action: setVisited, label: {
                    Text("Start burrowing")
                        .font(
                            .system(
                                size : 14,
                                weight: .regular,
                                design: .rounded))
                        .padding(.all, 30.0)
                        .foregroundColor(.white)
                }).buttonBorderShape(.roundedRectangle).buttonStyle(.borderless)
            }.padding(20.0)
        }
    }
}
